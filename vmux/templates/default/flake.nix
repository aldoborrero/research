{
  description = "vmux — microVM development environment";

  nixConfig = {
    extra-substituters = [
      "https://cache.numtide.com"
      "https://nix-community.cachix.org"
      "https://numtide.cachix.org"
      "https://cache.garnix.io"
    ];
    extra-trusted-public-keys = [
      "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "numtide.cachix.org-1:2ps1kLBUWjxIneOy1Ber+6DVv0PNFURkZB7pR+YhDv4w="
      "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    llm-agents = {
      url = "github:numtide/llm-agents.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    avim = {
      url = "github:aldoborrero/avim.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      microvm,
      llm-agents,
      avim,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ llm-agents.overlays.default ];
      };

      # ---------- Tune these ----------
      vmName = "dev";
      vmIp = "192.168.83.10";
      vmGateway = "192.168.83.1";
      vmMac = "52:54:00:12:34:56";
      hostWorkspace = "/home/user/project"; # absolute path on the host
      # ---------------------------------
    in
    {
      # The microVM NixOS configuration — used by `microvm -c` or declarative host import.
      nixosConfigurations.${vmName} = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          { nixpkgs.overlays = [ llm-agents.overlays.default ]; }
          {
            # ── Hypervisor ──────────────────────────────────────────────
            microvm = {
              hypervisor = "cloud-hypervisor";
              vcpu = 8;
              mem = 4096; # MiB

              # ── Networking ────────────────────────────────────────────
              interfaces = [
                {
                  type = "tap";
                  id = "vm-${vmName}";
                  mac = vmMac;
                }
              ];

              # ── Shared filesystems ────────────────────────────────────
              shares = [
                # Read-only nix store from host (avoids large VM closures).
                {
                  proto = "virtiofs";
                  tag = "ro-store";
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                }
                # Project workspace — read-write.
                {
                  proto = "virtiofs";
                  tag = "workspace";
                  source = hostWorkspace;
                  mountPoint = "/workspace";
                }
              ];
            };

            # ── Network inside guest ────────────────────────────────────
            systemd.network = {
              enable = true;
              networks."20-lan" = {
                matchConfig.Type = "ether";
                addresses = [
                  { Address = "${vmIp}/24"; }
                ];
                routes = [
                  { Gateway = vmGateway; }
                ];
              };
            };
            networking = {
              hostName = vmName;
              firewall.enable = false;
              useNetworkd = true;
            };

            # ── SSH ─────────────────────────────────────────────────────
            services.openssh = {
              enable = true;
              settings = {
                PermitRootLogin = "prohibit-password";
                PasswordAuthentication = false;
              };
            };

            # Add your public key(s) here so vmux can SSH in as root.
            users.users.root.openssh.authorizedKeys.keys = [
              # "ssh-ed25519 AAAA... you@host"
            ];

            # ── Dev tools & LLM agents ──────────────────────────────────
            environment.systemPackages =
              (with pkgs; [
                git
                curl
                jq
              ])
              ++ [
                avim.packages.${system}.avim
              ]
              ++ (with pkgs.llm-agents; [
                claude-code
                codex
              ]);

            system.stateVersion = "24.11";
          }
        ];
      };

      # ── Host module ───────────────────────────────────────────────────
      # Import this into your host NixOS configuration to register the VM.
      #
      #   imports = [ vmux-flake.nixosModules.host ];
      #
      nixosModules.host = {
        imports = [ microvm.nixosModules.host ];

        microvm.vms.${vmName} = {
          autostart = false; # vmux manages lifecycle
          flake = self;
        };

        # Bridge for microVM TAP interfaces.
        systemd.network = {
          enable = true;
          netdevs."10-microvm".netdevConfig = {
            Kind = "bridge";
            Name = "microbr";
          };
          networks."10-microvm" = {
            matchConfig.Name = "microbr";
            addresses = [
              { Address = "${vmGateway}/24"; }
            ];
          };
          networks."11-microvm" = {
            matchConfig.Name = "vm-*";
            networkConfig.Bridge = "microbr";
          };
        };

        # NAT so the VM can reach the internet.
        networking.nat = {
          enable = true;
          internalInterfaces = [ "microbr" ];
        };
      };
    };
}
