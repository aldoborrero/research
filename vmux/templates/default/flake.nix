{
  description = "vmux — microVM development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      microvm,
    }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

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

            # ── Dev tools ───────────────────────────────────────────────
            environment.systemPackages = with pkgs; [
              git
              neovim
              helix
              curl
              jq
            ];

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
