import requests
import json
import time
import socket
import ssl
import ipaddress
from collections import defaultdict
# ── RIPE NCC ──────────────────────────────────────────────────────────────────
def buscar_ripe(query: str, delay: float = 1.0):
    """Busca rangos de IP en RIPE NCC.

    RIPE search API treats multi-word strings as multiple lookup keys.
    For org name searches, single keywords work best (e.g. "Indra" not
    "Indra Sistemas"). For exact ranges use IPs/AS numbers directly.
    """
    url = "https://rest.db.ripe.net/search.json"
    params = {
        "query-string": query,
        "type-filter": "inetnum,inet6num,aut-num",
        "source": "RIPE",
        "flags": "no-referenced",
    }
    headers = {"Accept": "application/json"}

    try:
        r = requests.get(url, params=params, headers=headers, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] RIPE error para '{query}': {e}")
        return []

    data = r.json()

    resultados = []
    CAMPOS = {"inetnum", "inet6num", "aut-num", "netname", "descr", "org", "country", "status"}

    for obj in data.get("objects", {}).get("object", []):
        entry = defaultdict(list)
        for attr in obj.get("attributes", {}).get("attribute", []):
            if attr["name"] in CAMPOS:
                entry[attr["name"]].append(attr["value"])

        final = {k: v[0] if len(v) == 1 else v for k, v in entry.items()}
        if final:
            resultados.append(final)

    time.sleep(delay)
    return resultados
# ── RIPE STAT ─────────────────────────────────────────────────────────────────
def buscar_ripestat_prefix(prefix: str):
    """Consulta RIPE Stat para un prefijo: routing, geoloc, abuse contacts."""
    base = "https://stat.ripe.net/data"
    resultados = {}

    # Routing status
    try:
        r = requests.get(f"{base}/routing-status/data.json",
                         params={"resource": prefix}, timeout=15)
        r.raise_for_status()
        data = r.json().get("data", {})
        resultados["routing"] = {
            "status": data.get("status"),
            "first_seen": data.get("first_seen"),
            "last_seen": data.get("last_seen"),
            "visibility": data.get("visibility", {}).get("v4", {}).get("total_peers"),
        }
    except requests.RequestException as e:
        print(f"  [!] RIPE Stat routing error para '{prefix}': {e}")

    time.sleep(0.5)

    # Geolocation
    try:
        r = requests.get(f"{base}/maxmind-geo-lite/data.json",
                         params={"resource": prefix}, timeout=15)
        r.raise_for_status()
        locs = r.json().get("data", {}).get("located_resources", [])
        resultados["geolocation"] = [
            {
                "prefix": loc.get("resource"),
                "country": loc.get("locations", [{}])[0].get("country") if loc.get("locations") else None,
                "city": loc.get("locations", [{}])[0].get("city") if loc.get("locations") else None,
            }
            for loc in locs
        ]
    except requests.RequestException as e:
        print(f"  [!] RIPE Stat geo error para '{prefix}': {e}")

    time.sleep(0.5)

    # Abuse contacts
    try:
        r = requests.get(f"{base}/abuse-contact-finder/data.json",
                         params={"resource": prefix}, timeout=15)
        r.raise_for_status()
        data = r.json().get("data", {})
        resultados["abuse_contacts"] = data.get("abuse_contacts", [])
        resultados["authoritative_rir"] = data.get("authoritative_rir")
    except requests.RequestException as e:
        print(f"  [!] RIPE Stat abuse error para '{prefix}': {e}")

    time.sleep(0.5)

    # Network info (holder, ASN)
    try:
        r = requests.get(f"{base}/network-info/data.json",
                         params={"resource": prefix}, timeout=15)
        r.raise_for_status()
        data = r.json().get("data", {})
        resultados["network_info"] = {
            "asns": data.get("asns", []),
            "prefix": data.get("prefix"),
        }
    except requests.RequestException as e:
        print(f"  [!] RIPE Stat network error para '{prefix}': {e}")

    return resultados
# ── CRT.SH (Certificate Transparency) ────────────────────────────────────────
def buscar_crtsh(domain: str):
    """Busca subdominios en Certificate Transparency logs via crt.sh."""
    url = "https://crt.sh/"
    params = {"q": f"%.{domain}", "output": "json"}

    try:
        r = requests.get(url, params=params, timeout=30)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] crt.sh error para '{domain}': {e}")
        return []

    try:
        entries = r.json()
    except json.JSONDecodeError:
        print(f"  [!] crt.sh respuesta inválida para '{domain}'")
        return []

    # Dedup and extract unique names
    nombres = set()
    certs = []
    for entry in entries:
        name_value = entry.get("name_value", "")
        issuer = entry.get("issuer_name", "")
        not_before = entry.get("not_before", "")
        not_after = entry.get("not_after", "")

        for name in name_value.split("\n"):
            name = name.strip().lower()
            if name and name not in nombres:
                nombres.add(name)
                certs.append({
                    "subdomain": name,
                    "issuer": issuer,
                    "not_before": not_before,
                    "not_after": not_after,
                })

    return certs
# ── REVERSE DNS ───────────────────────────────────────────────────────────────
def _parse_inetnum(inetnum: str):
    """Convierte 'x.x.x.x - y.y.y.y' a lista de IPs (max 256 para no abusar)."""
    parts = inetnum.split(" - ")
    if len(parts) != 2:
        return []
    try:
        start = ipaddress.IPv4Address(parts[0].strip())
        end = ipaddress.IPv4Address(parts[1].strip())
    except ipaddress.AddressValueError:
        return []

    count = int(end) - int(start) + 1
    if count > 256:
        # Solo primeras y últimas para rangos grandes
        ips = [start + i for i in range(min(16, count))]
        ips += [end - i for i in range(min(16, count - 16))]
        return list(set(ips))
    return [start + i for i in range(count)]

def reverse_dns(inetnum: str, max_ips: int = 64):
    """Hace reverse DNS (PTR) en un rango inetnum."""
    ips = _parse_inetnum(inetnum)[:max_ips]
    resultados = []

    for ip in ips:
        try:
            hostname, _, _ = socket.gethostbyaddr(str(ip))
            resultados.append({"ip": str(ip), "ptr": hostname})
        except (socket.herror, socket.gaierror, OSError):
            pass

    return resultados
# ── DNS RECORDS (via Cloudflare DoH) ──────────────────────────────────────────
def _doh_query(name: str, rtype: str):
    """Consulta DNS sobre HTTPS (Cloudflare) — no necesita dig."""
    try:
        r = requests.get(
            "https://cloudflare-dns.com/dns-query",
            params={"name": name, "type": rtype},
            headers={"Accept": "application/dns-json"},
            timeout=10,
        )
        r.raise_for_status()
        answers = r.json().get("Answer", [])
        return [a["data"] for a in answers if a.get("data")]
    except requests.RequestException:
        return []

def consultar_dns(domain: str):
    """Consulta registros DNS relevantes para reconocimiento via DoH."""
    registros = {}
    tipos = ["A", "AAAA", "MX", "NS", "TXT", "SOA", "CNAME"]

    for tipo in tipos:
        lines = _doh_query(domain, tipo)
        if lines:
            registros[tipo] = lines
        time.sleep(0.2)

    # Extraer info de seguridad de TXT records
    seguridad = {}
    for txt in registros.get("TXT", []):
        txt_lower = txt.lower()
        if "v=spf1" in txt_lower:
            seguridad["SPF"] = txt
        elif "v=dmarc1" in txt_lower:
            seguridad["DMARC"] = txt
        elif "v=dkim1" in txt_lower:
            seguridad["DKIM"] = txt

    # DMARC vive en _dmarc.domain
    dmarc_lines = _doh_query(f"_dmarc.{domain}", "TXT")
    if dmarc_lines:
        seguridad["DMARC"] = dmarc_lines[0]

    registros["security_policies"] = seguridad
    return registros
# ── PORT SCANNING (netcat / socket fallback) ──────────────────────────────────
PUERTOS_COMUNES = [
    21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445, 465, 587,
    993, 995, 1433, 1521, 2049, 3306, 3389, 5432, 5900, 6379, 8000, 8080,
    8443, 8888, 9090, 9200, 9443, 27017,
]

def _nc_disponible():
    """Comprueba si netcat está disponible."""
    import subprocess, shutil
    return shutil.which("nc") is not None

def _scan_nc(ip: str, puerto: int, timeout: float = 2.0):
    """Escanea un puerto con netcat (nc -zw)."""
    import subprocess
    try:
        result = subprocess.run(
            ["nc", "-z", "-w", str(int(timeout)), ip, str(puerto)],
            capture_output=True, timeout=timeout + 1,
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        return False

def _scan_socket(ip: str, puerto: int, timeout: float = 2.0):
    """Escanea un puerto con socket (fallback si nc no está)."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    try:
        sock.connect((ip, puerto))
        sock.close()
        return True
    except (socket.timeout, ConnectionRefusedError, OSError):
        sock.close()
        return False

def _grab_banner(ip: str, puerto: int, timeout: float = 3.0):
    """Intenta obtener el banner del servicio."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    try:
        sock.connect((ip, puerto))
        # Para HTTP, enviar request mínimo
        if puerto in (80, 8080, 8000, 8888, 9090):
            sock.sendall(f"HEAD / HTTP/1.0\r\nHost: {ip}\r\n\r\n".encode())
        elif puerto in (443, 8443, 9443):
            sock.close()
            return _grab_https_banner(ip, puerto, timeout)
        else:
            # Muchos servicios envían banner al conectar (SSH, FTP, SMTP...)
            sock.sendall(b"\r\n")
        data = sock.recv(1024)
        sock.close()
        return data.decode("utf-8", errors="replace").strip()[:200]
    except (socket.timeout, ConnectionRefusedError, OSError):
        sock.close()
        return None

def _grab_https_banner(ip: str, puerto: int, timeout: float = 3.0):
    """Obtiene info del certificado SSL/TLS."""
    import ssl
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    try:
        with socket.create_connection((ip, puerto), timeout=timeout) as raw:
            with ctx.wrap_socket(raw, server_hostname=ip) as ssock:
                cert = ssock.getpeercert(binary_form=False)
                if cert:
                    subject = dict(x[0] for x in cert.get("subject", []))
                    issuer = dict(x[0] for x in cert.get("issuer", []))
                    return f"CN={subject.get('commonName','?')} Issuer={issuer.get('organizationName','?')}"
                # If no parsed cert, try cipher info
                cipher = ssock.cipher()
                if cipher:
                    return f"TLS {cipher[1]} {cipher[0]}"
    except Exception:
        pass
    return None

def _scan_scapy_syn(ip: str, puertos: list, timeout: float = 2.0):
    """SYN scan con scapy (half-open, más sigiloso). Requiere root."""
    try:
        from scapy.all import IP, TCP, sr, conf
        conf.verb = 0
    except ImportError:
        return None  # scapy no disponible

    try:
        pkt = IP(dst=ip) / TCP(dport=puertos, flags="S")
        answered, _ = sr(pkt, timeout=timeout, verbose=0)
    except PermissionError:
        return None  # necesita root
    except Exception:
        return None

    abiertos = []
    for sent, received in answered:
        if received.haslayer(TCP) and received[TCP].flags == 0x12:  # SYN-ACK
            abiertos.append(received[TCP].sport)
    return abiertos

def escanear_puertos(ip: str, puertos: list = None, timeout: float = 2.0,
                     banner: bool = True, metodo_forzado: str = None):
    """Escanea puertos en una IP.

    Orden de preferencia: scapy SYN (si root) > netcat > socket.
    metodo_forzado: 'scapy', 'nc', 'socket' para forzar backend.
    """
    if puertos is None:
        puertos = PUERTOS_COMUNES

    # Intentar scapy SYN scan primero (más rápido, escanea todos los puertos en paralelo)
    if metodo_forzado in (None, "scapy"):
        syn_result = _scan_scapy_syn(ip, puertos, timeout)
        if syn_result is not None:
            abiertos = []
            for puerto in syn_result:
                entry = {"puerto": puerto, "metodo": "scapy-syn"}
                if banner:
                    b = _grab_banner(ip, puerto, timeout + 1)
                    if b:
                        entry["banner"] = b
                abiertos.append(entry)
            return abiertos

    # Fallback: nc o socket (secuencial)
    if metodo_forzado == "nc" or (metodo_forzado is None and _nc_disponible()):
        scan_fn = _scan_nc
        metodo = "nc"
    else:
        scan_fn = _scan_socket
        metodo = "socket"

    abiertos = []
    for puerto in puertos:
        if scan_fn(ip, puerto, timeout):
            entry = {"puerto": puerto, "metodo": metodo}
            if banner:
                b = _grab_banner(ip, puerto, timeout + 1)
                if b:
                    entry["banner"] = b
            abiertos.append(entry)

    return abiertos
# ── WAYBACK MACHINE (CDX API) ─────────────────────────────────────────────────
def buscar_wayback(domain: str, max_results: int = 500):
    """Consulta el CDX API de Wayback Machine para un dominio.

    Devuelve URLs históricas, timestamps, status codes y MIME types.
    Útil para encontrar endpoints eliminados, paneles de admin, APIs antiguas.
    """
    url = "https://web.archive.org/cdx/search/cdx"
    params = {
        "url": f"*.{domain}/*",
        "output": "json",
        "fl": "timestamp,original,mimetype,statuscode,digest",
        "collapse": "urlkey",  # dedup por URL normalizada
        "limit": max_results,
        "filter": "statuscode:200",
    }

    try:
        r = requests.get(url, params=params, timeout=30)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] Wayback error para '{domain}': {e}")
        return {}

    try:
        rows = r.json()
    except json.JSONDecodeError:
        print(f"  [!] Wayback respuesta inválida para '{domain}'")
        return {}

    if not rows or len(rows) < 2:
        return {"urls": [], "stats": {}}

    header = rows[0]
    entries = rows[1:]

    urls = []
    subdominios = set()
    mimetypes = defaultdict(int)
    extensiones = defaultdict(int)
    rutas_interesantes = []

    RUTAS_SENSIBLES = {
        "admin", "login", "api", "console", "dashboard", "config",
        "backup", "staging", "test", "debug", "internal", "portal",
        "phpmyadmin", "wp-admin", "jenkins", "gitlab", "grafana",
        "kibana", "swagger", "graphql", ".env", ".git", "actuator",
        "server-status", "server-info", "web.config", "robots.txt",
        "sitemap.xml", ".well-known",
    }

    for row in entries:
        timestamp = row[0]
        original_url = row[1]
        mimetype = row[2]
        statuscode = row[3]
        digest = row[4]

        urls.append({
            "timestamp": timestamp,
            "url": original_url,
            "mimetype": mimetype,
            "status": statuscode,
        })

        # Extraer subdominio
        try:
            from urllib.parse import urlparse
            parsed = urlparse(original_url if "://" in original_url else f"http://{original_url}")
            if parsed.hostname:
                subdominios.add(parsed.hostname.lower())
            path_lower = parsed.path.lower()
        except Exception:
            path_lower = original_url.lower()

        mimetypes[mimetype] += 1

        # Extensión del archivo
        if "." in original_url.split("/")[-1]:
            ext = original_url.split(".")[-1].split("?")[0].lower()[:10]
            extensiones[ext] += 1

        # Rutas interesantes
        for ruta in RUTAS_SENSIBLES:
            if ruta in path_lower:
                rutas_interesantes.append({
                    "url": original_url,
                    "match": ruta,
                    "timestamp": timestamp,
                })
                break

    # Dedup rutas interesantes por URL
    seen = set()
    rutas_dedup = []
    for r in rutas_interesantes:
        if r["url"] not in seen:
            seen.add(r["url"])
            rutas_dedup.append(r)

    return {
        "total_urls": len(urls),
        "rango_temporal": {
            "primera_captura": entries[0][0] if entries else None,
            "ultima_captura": entries[-1][0] if entries else None,
        },
        "subdominios_historicos": sorted(subdominios),
        "mimetypes": dict(mimetypes),
        "extensiones_top": dict(sorted(extensiones.items(), key=lambda x: -x[1])[:20]),
        "rutas_interesantes": rutas_dedup,
        "urls": urls,
    }
# ── BGPVIEW (alternativa gratuita a Shodan para ASNs) ─────────────────────────
def buscar_bgpview_asn(asn: str):
    """Obtiene prefijos anunciados por un ASN. BGPView es gratuito y sin key."""
    url = f"https://api.bgpview.io/asn/{asn}/prefixes"

    try:
        r = requests.get(url, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] BGPView ASN error para '{asn}': {e}")
        return []

    data = r.json()

    prefijos = []
    for p in data.get("data", {}).get("ipv4_prefixes", []):
        prefijos.append({
            "prefix": p.get("prefix"),
            "name": p.get("name"),
            "description": p.get("description"),
            "country": p.get("country_code"),
            "parent_prefix": p.get("parent", {}).get("prefix"),
        })
    return prefijos
def buscar_bgpview_org(nombre: str):
    """Busca ASNs por nombre de organización."""
    url = "https://api.bgpview.io/search"

    try:
        r = requests.get(url, params={"query_term": nombre}, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] BGPView org error para '{nombre}': {e}")
        return []

    data = r.json()

    asns = []
    for asn in data.get("data", {}).get("asns", []):
        asns.append({
            "asn": asn.get("asn"),
            "name": asn.get("name"),
            "description": asn.get("description"),
            "country": asn.get("country_code"),
        })
    return asns
# ── TLS CERTIFICATE SAN ANALYSIS ──────────────────────────────────────────────
STAGING_PATTERNS = [
    "staging", "stage", "stg", "dev", "develop", "test", "testing", "tst",
    "preprod", "pre-prod", "pre.", "uat", "qa", "beta", "sandbox", "demo",
    "int.", "internal", "intranet", "corp", "lab", "pilot", "canary",
    "alpha", "gamma", "preview", "draft", "tmp", "temp", "old", "legacy",
    "backup", "bak", "dr.", "disaster", "mirror", "secondary",
]

def _extraer_san_cert(host: str, puerto: int = 443, timeout: float = 5.0):
    """Conecta a host:puerto via TLS y extrae Subject + SANs del certificado.

    Devuelve dict con CN, SANs, issuer, validity, y cipher info.
    No valida el certificado (queremos ver certs autofirmados de staging).
    """
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE

    try:
        with socket.create_connection((host, puerto), timeout=timeout) as raw:
            with ctx.wrap_socket(raw, server_hostname=host) as ssock:
                # getpeercert(binary_form=False) needs CERT_REQUIRED for full parse,
                # but with CERT_NONE it returns {} — so use binary + ssl helpers
                der_cert = ssock.getpeercert(binary_form=True)
                parsed = ssock.getpeercert(binary_form=False)
                cipher_info = ssock.cipher()
                version = ssock.version()

                # With CERT_NONE, parsed is usually empty. Try the binary path.
                san_names = set()
                cn = None
                issuer_org = None
                not_before = None
                not_after = None

                if parsed:
                    # Extract CN
                    for rdn in parsed.get("subject", ()):
                        for attr_name, attr_val in rdn:
                            if attr_name == "commonName":
                                cn = attr_val
                                san_names.add(attr_val.lower())

                    # Extract SANs
                    for san_type, san_val in parsed.get("subjectAltName", ()):
                        if san_type == "DNS":
                            san_names.add(san_val.lower())

                    # Issuer
                    for rdn in parsed.get("issuer", ()):
                        for attr_name, attr_val in rdn:
                            if attr_name == "organizationName":
                                issuer_org = attr_val

                    not_before = parsed.get("notBefore")
                    not_after = parsed.get("notAfter")

                # Fallback: parse DER with ssl module
                if der_cert and not san_names:
                    try:
                        # Re-connect with CERT_REQUIRED to a temp context for parsing
                        ctx2 = ssl.create_default_context()
                        ctx2.check_hostname = False
                        ctx2.verify_mode = ssl.CERT_REQUIRED
                        # Load the cert we already have
                        # Actually, simplest: use openssl-style parsing via ssl
                        pem = ssl.DER_cert_to_PEM_cert(der_cert)
                        # ssl module doesn't expose SAN parsing from PEM directly
                        # but we can re-connect with verify to get parsed cert
                        pass
                    except Exception:
                        pass

                # If we still have no SANs, try a second connection with partial verify
                if not san_names:
                    try:
                        ctx3 = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
                        ctx3.check_hostname = False
                        ctx3.verify_mode = ssl.CERT_OPTIONAL
                        with socket.create_connection((host, puerto), timeout=timeout) as raw2:
                            with ctx3.wrap_socket(raw2, server_hostname=host) as ssock2:
                                parsed2 = ssock2.getpeercert(binary_form=False)
                                if parsed2:
                                    for rdn in parsed2.get("subject", ()):
                                        for attr_name, attr_val in rdn:
                                            if attr_name == "commonName":
                                                cn = attr_val
                                                san_names.add(attr_val.lower())
                                    for san_type, san_val in parsed2.get("subjectAltName", ()):
                                        if san_type == "DNS":
                                            san_names.add(san_val.lower())
                                    for rdn in parsed2.get("issuer", ()):
                                        for attr_name, attr_val in rdn:
                                            if attr_name == "organizationName":
                                                issuer_org = attr_val
                                    not_before = parsed2.get("notBefore")
                                    not_after = parsed2.get("notAfter")
                    except Exception:
                        pass

                return {
                    "host": host,
                    "port": puerto,
                    "cn": cn,
                    "san_names": sorted(san_names),
                    "issuer": issuer_org,
                    "not_before": not_before,
                    "not_after": not_after,
                    "tls_version": version,
                    "cipher": cipher_info[0] if cipher_info else None,
                }
    except (socket.timeout, ConnectionRefusedError, OSError, ssl.SSLError) as e:
        return {"host": host, "port": puerto, "error": str(e)}


def analizar_san(dominios_conocidos: set, hosts_a_probar: list,
                 puertos_tls: list = None, timeout: float = 5.0):
    """Analiza certificados TLS de múltiples hosts y extrae SANs.

    - dominios_conocidos: set de dominios ya descubiertos (crt.sh, DNS, etc.)
    - hosts_a_probar: lista de IPs/hostnames a conectar
    - puertos_tls: puertos HTTPS a probar por host (default: [443, 8443, 9443])

    Devuelve:
    - certs: lista de cert info por host
    - san_nuevos: SANs no vistos en dominios_conocidos
    - san_staging: SANs que matchean patrones staging/dev/test
    - cohosted: dominios que comparten certificado (misma infra)
    """
    if puertos_tls is None:
        puertos_tls = [443, 8443, 9443]

    certs = []
    todos_san = set()
    # Map: SAN -> set of hosts that serve it (for co-hosting detection)
    san_a_hosts = defaultdict(set)

    probed = set()
    for host in hosts_a_probar:
        for puerto in puertos_tls:
            key = (host, puerto)
            if key in probed:
                continue
            probed.add(key)

            info = _extraer_san_cert(host, puerto, timeout)
            if info.get("error"):
                continue

            sans = info.get("san_names", [])
            if not sans:
                continue

            certs.append(info)
            for san in sans:
                # Remove wildcard prefix for matching
                clean = san.lstrip("*.")
                todos_san.add(clean)
                san_a_hosts[clean].add(host)

    # Normalize known domains
    conocidos_norm = {d.lower().lstrip("*.") for d in dominios_conocidos}

    # New SANs not in known set
    san_nuevos = sorted(todos_san - conocidos_norm)

    # SANs matching staging patterns
    san_staging = []
    for san in sorted(todos_san):
        san_lower = san.lower()
        for pattern in STAGING_PATTERNS:
            if pattern in san_lower:
                san_staging.append({"san": san, "pattern": pattern})
                break

    # Co-hosted: SANs served by multiple hosts
    cohosted = {}
    for san, hosts in san_a_hosts.items():
        if len(hosts) > 1:
            cohosted[san] = sorted(hosts)

    # Shared certs: group hosts that serve identical SAN sets
    cert_groups = defaultdict(list)
    for cert in certs:
        san_key = tuple(cert.get("san_names", []))
        if san_key:
            cert_groups[san_key].append(f"{cert['host']}:{cert['port']}")

    shared_certs = {
        ", ".join(sorted(sans)): endpoints
        for sans, endpoints in cert_groups.items()
        if len(endpoints) > 1
    }

    return {
        "certs": certs,
        "total_unique_sans": len(todos_san),
        "san_nuevos": san_nuevos,
        "san_staging": san_staging,
        "cohosted": cohosted,
        "shared_certs": shared_certs,
    }
# ── VIRUSTOTAL (passive DNS / subdomains) ─────────────────────────────────────
def buscar_virustotal(domain: str, api_key: str, max_results: int = 200):
    """Consulta VirusTotal para subdominios via relación 'subdomains'.

    Free tier: 4 requests/min, 500/day. No necesita cuenta premium para subdominios.
    Devuelve subdominios + últimas resoluciones DNS pasivas.
    """
    url = f"https://www.virustotal.com/api/v3/domains/{domain}/subdomains"
    headers = {"x-apikey": api_key}
    params = {"limit": min(max_results, 40)}  # VT pages at max 40

    subdominios = []
    cursor = None

    while len(subdominios) < max_results:
        if cursor:
            params["cursor"] = cursor

        try:
            r = requests.get(url, headers=headers, params=params, timeout=20)
            if r.status_code == 429:
                print("  [!] VirusTotal rate limit — esperando 60s...")
                time.sleep(60)
                continue
            r.raise_for_status()
        except requests.RequestException as e:
            print(f"  [!] VirusTotal error para '{domain}': {e}")
            break

        data = r.json()

        for item in data.get("data", []):
            sub_id = item.get("id", "")
            attrs = item.get("attributes", {})
            last_dns = attrs.get("last_dns_records", [])

            entry = {
                "subdomain": sub_id,
                "last_modification_date": attrs.get("last_modification_date"),
                "dns_records": [
                    {"type": rec.get("type"), "value": rec.get("value")}
                    for rec in last_dns[:10]
                ],
            }
            subdominios.append(entry)

        cursor = data.get("meta", {}).get("cursor")
        if not cursor or not data.get("data"):
            break

        time.sleep(15)  # Respetar rate limit free tier (4 req/min)

    # También consultar resoluciones DNS pasivas (IPs históricas)
    resoluciones = []
    res_url = f"https://www.virustotal.com/api/v3/domains/{domain}/resolutions"
    try:
        r = requests.get(res_url, headers=headers, params={"limit": 40}, timeout=20)
        if r.status_code != 429:
            r.raise_for_status()
            for item in r.json().get("data", []):
                attrs = item.get("attributes", {})
                resoluciones.append({
                    "ip": attrs.get("ip_address"),
                    "date": attrs.get("date"),
                    "host_name": attrs.get("host_name"),
                })
    except requests.RequestException as e:
        print(f"  [!] VirusTotal resoluciones error: {e}")

    return {
        "subdominios": subdominios,
        "resoluciones_pasivas": resoluciones,
        "total_subdominios": len(subdominios),
    }
# ── SECURITYTRAILS (subdomain enumeration) ────────────────────────────────────
def buscar_securitytrails(domain: str, api_key: str):
    """Consulta SecurityTrails para subdominios y DNS histórico.

    Free tier: 50 requests/month. Devuelve subdominios + historial DNS (A, AAAA, MX, NS).
    """
    headers = {"APIKEY": api_key, "Accept": "application/json"}
    resultados = {"subdominios": [], "dns_history": {}}

    # Subdomain listing
    url = f"https://api.securitytrails.com/v1/domain/{domain}/subdomains"
    params = {"children_only": "false", "include_inactive": "true"}

    try:
        r = requests.get(url, headers=headers, params=params, timeout=20)
        if r.status_code == 429:
            print("  [!] SecurityTrails rate limit")
            return resultados
        r.raise_for_status()
        data = r.json()

        for sub in data.get("subdomains", []):
            fqdn = f"{sub}.{domain}"
            resultados["subdominios"].append(fqdn)

        resultados["endpoint_count"] = data.get("endpoint_count", 0)
        # Also include the subdomain count from their data
        resultados["subdomain_count"] = data.get("subdomain_count", len(resultados["subdominios"]))
    except requests.RequestException as e:
        print(f"  [!] SecurityTrails subdomains error para '{domain}': {e}")

    time.sleep(1)

    # DNS history (A records) — shows IP changes over time, useful to find old/staging IPs
    for record_type in ["a", "aaaa", "mx", "ns"]:
        hist_url = f"https://api.securitytrails.com/v1/history/{domain}/dns/{record_type}"
        try:
            r = requests.get(hist_url, headers=headers, timeout=20)
            if r.status_code == 429:
                print(f"  [!] SecurityTrails rate limit en DNS history ({record_type})")
                break
            r.raise_for_status()
            data = r.json()

            records = []
            for record in data.get("records", []):
                values = record.get("values", [])
                records.append({
                    "first_seen": record.get("first_seen"),
                    "last_seen": record.get("last_seen"),
                    "organizations": record.get("organizations", []),
                    "values": [v.get("ip", v.get("value", "")) for v in values],
                })
            resultados["dns_history"][record_type.upper()] = records
        except requests.RequestException as e:
            print(f"  [!] SecurityTrails DNS history error ({record_type}): {e}")

        time.sleep(1)

    return resultados
# ── SHODAN ────────────────────────────────────────────────────────────────────
def buscar_shodan(query: str, api_key: str, max_paginas: int = 1):
    """Consulta Shodan con paginación correcta."""
    import shodan

    api = shodan.Shodan(api_key)
    resultados = []

    try:
        primer_resultado = api.search(query, page=1)
        total = primer_resultado["total"]
        print(f"  Total: {total} resultados")

        paginas_disponibles = min(max_paginas, (total // 100) + 1)

        for pagina in range(1, paginas_disponibles + 1):
            if pagina > 1:
                resultado = api.search(query, page=pagina)
                time.sleep(1)
            else:
                resultado = primer_resultado

            if not resultado["matches"]:
                break

            for host in resultado["matches"]:
                resultados.append({
                    "ip": host.get("ip_str"),
                    "puerto": host.get("port"),
                    "org": host.get("org"),
                    "hostnames": host.get("hostnames", []),
                    "titulo": host.get("http", {}).get("title"),
                    "server": host.get("http", {}).get("server"),
                    "pais": host.get("location", {}).get("country_name"),
                    "ciudad": host.get("location", {}).get("city"),
                    "cpe": host.get("cpe", []),
                    "vulns": list(host.get("vulns", {}).keys()),
                    "timestamp": host.get("timestamp"),
                })

    except shodan.APIError as e:
        print(f"  Error Shodan: {e}")

    return resultados
# ── MAIN ──────────────────────────────────────────────────────────────────────
TARGETS = {
    "DGT": {
        "domains": ["dgt.es", "dgt.gob.es"],
        "ripe_queries": ["DGT"],
        "bgpview_queries": ["Direccion General de Trafico", "DGT"],
        "shodan_queries": [
            'org:"Direccion General de Trafico"',
            'ssl:"dgt.es" country:ES',
            'hostname:".dgt.es"',
        ],
    },
    "Indra": {
        "domains": ["indracompany.com", "indra.es", "minsait.com"],
        "ripe_queries": ["Indra"],
        "bgpview_queries": ["Indra Sistemas"],
        "shodan_queries": [
            'org:"Indra Sistemas" country:ES',
            'http.title:"ITS" country:ES org:"Indra"',
            'ssl:"indra.es" country:ES',
        ],
    },
    "AEAT": {
        "domains": ["agenciatributaria.es", "agenciatributaria.gob.es", "aeat.es"],
        "ripe_queries": ["AEAT"],
        "bgpview_queries": ["Agencia Tributaria"],
        "shodan_queries": [
            'hostname:".aeat.es"',
            'ssl:"agenciatributaria.es"',
        ],
    },
}
if __name__ == "__main__":
    import argparse
    import sys

    parser = argparse.ArgumentParser(
        description="OSINT reconnaissance: RIPE, BGPView, crt.sh, DNS, RIPE Stat, VirusTotal, SecurityTrails, Shodan"
    )
    parser.add_argument("--shodan-key", default=None)
    parser.add_argument("--vt-key", default=None, help="VirusTotal API key (free tier: 4 req/min)")
    parser.add_argument("--st-key", default=None, help="SecurityTrails API key (free tier: 50 req/month)")
    parser.add_argument("--target", choices=list(TARGETS.keys()) + ["all"], default="all")
    parser.add_argument("--output", default="resultados.json")
    parser.add_argument("--skip-rdns", action="store_true", help="Saltar reverse DNS (lento)")
    parser.add_argument("--skip-ripestat", action="store_true", help="Saltar RIPE Stat (rate limited)")
    parser.add_argument("--skip-wayback", action="store_true", help="Saltar Wayback Machine")
    parser.add_argument("--skip-portscan", action="store_true", help="Saltar port scanning")
    parser.add_argument("--skip-san", action="store_true", help="Saltar TLS SAN analysis")
    parser.add_argument("--san-timeout", type=float, default=5.0, help="TLS connect timeout for SAN extraction (default: 5)")
    parser.add_argument("--san-ports", default="443,8443,9443", help="TLS ports for SAN probing (default: 443,8443,9443)")
    parser.add_argument("--scan-timeout", type=float, default=2.0, help="Timeout por puerto en segundos (default: 2)")
    parser.add_argument("--scan-ports", default=None, help="Puertos a escanear (ej: '22,80,443,8080')")
    parser.add_argument("--scan-method", choices=["auto", "scapy", "nc", "socket"], default="auto",
                        help="Método de escaneo (default: auto = scapy > nc > socket)")
    args = parser.parse_args()

    if args.scan_ports:
        PUERTOS_COMUNES.clear()
        PUERTOS_COMUNES.extend(int(p.strip()) for p in args.scan_ports.split(","))

    targets = TARGETS if args.target == "all" else {args.target: TARGETS[args.target]}
    todos = {}

    for nombre, config in targets.items():
        print(f"\n{'='*60}")
        print(f"TARGET: {nombre}")
        print(f"{'='*60}")

        todos[nombre] = {
            "ripe": [], "bgpview": [], "shodan": [],
            "crtsh": [], "dns": {}, "ripestat": {}, "reverse_dns": [],
            "wayback": {}, "portscan": {},
            "virustotal": {}, "securitytrails": {}, "tls_san": {},
        }

        # ── RIPE NCC ──
        print("\n[RIPE NCC]")
        for q in config["ripe_queries"]:
            print(f"  Query: {q}")
            res = buscar_ripe(q)
            todos[nombre]["ripe"].extend(res)
            for r in res:
                print(f"    {r.get('inetnum', r.get('aut-num', '?'))} - {r.get('netname', '?')} ({r.get('country', '?')})")

        # ── crt.sh (Certificate Transparency) ──
        print("\n[crt.sh - Certificate Transparency]")
        for domain in config.get("domains", []):
            print(f"  Dominio: {domain}")
            certs = buscar_crtsh(domain)
            todos[nombre]["crtsh"].extend(certs)
            subdominios_unicos = sorted(set(c["subdomain"] for c in certs))
            print(f"    {len(subdominios_unicos)} subdominios únicos encontrados")
            for s in subdominios_unicos[:20]:
                print(f"    {s}")
            if len(subdominios_unicos) > 20:
                print(f"    ... y {len(subdominios_unicos) - 20} más")
            time.sleep(2)  # crt.sh rate limit

        # ── DNS Records ──
        print("\n[DNS Records]")
        for domain in config.get("domains", []):
            print(f"  Dominio: {domain}")
            dns_info = consultar_dns(domain)
            todos[nombre]["dns"][domain] = dns_info
            for tipo, valores in dns_info.items():
                if tipo == "security_policies":
                    for pol, val in valores.items():
                        print(f"    {pol}: {val}")
                else:
                    for v in (valores if isinstance(valores, list) else [valores]):
                        print(f"    {tipo}: {v}")

        # ── Wayback Machine ──
        if not args.skip_wayback:
            print("\n[Wayback Machine]")
            for domain in config.get("domains", []):
                print(f"  Dominio: {domain}")
                wb = buscar_wayback(domain)
                todos[nombre]["wayback"][domain] = wb
                if wb.get("total_urls"):
                    rango = wb.get("rango_temporal", {})
                    print(f"    URLs únicas: {wb['total_urls']}")
                    print(f"    Rango: {rango.get('primera_captura', '?')} - {rango.get('ultima_captura', '?')}")
                    print(f"    Subdominios históricos: {len(wb.get('subdominios_historicos', []))}")
                    for s in wb.get("subdominios_historicos", [])[:10]:
                        print(f"      {s}")
                    if len(wb.get("subdominios_historicos", [])) > 10:
                        print(f"      ... y {len(wb['subdominios_historicos']) - 10} más")
                    print(f"    Extensiones top: {wb.get('extensiones_top', {})}")
                    rutas = wb.get("rutas_interesantes", [])
                    if rutas:
                        print(f"    Rutas interesantes ({len(rutas)}):")
                        for ri in rutas[:15]:
                            print(f"      [{ri['match']}] {ri['url']} ({ri['timestamp']})")
                        if len(rutas) > 15:
                            print(f"      ... y {len(rutas) - 15} más")
                else:
                    print(f"    Sin resultados")
                time.sleep(1)

        # ── VirusTotal ──
        if args.vt_key:
            print("\n[VirusTotal - Passive DNS]")
            for domain in config.get("domains", []):
                print(f"  Dominio: {domain}")
                vt = buscar_virustotal(domain, args.vt_key)
                todos[nombre]["virustotal"][domain] = vt
                print(f"    Subdominios: {vt['total_subdominios']}")
                for sub in vt.get("subdominios", [])[:20]:
                    dns_str = ", ".join(f"{r['type']}={r['value']}" for r in sub.get("dns_records", [])[:3])
                    print(f"    {sub['subdomain']}" + (f"  ({dns_str})" if dns_str else ""))
                if vt["total_subdominios"] > 20:
                    print(f"    ... y {vt['total_subdominios'] - 20} más")
                resol = vt.get("resoluciones_pasivas", [])
                if resol:
                    print(f"    Resoluciones pasivas: {len(resol)}")
                    for res in resol[:10]:
                        print(f"      {res.get('host_name', '?')} -> {res.get('ip', '?')} ({res.get('date', '?')})")
                time.sleep(15)  # VT rate limit
        else:
            print("\n[VirusTotal] Saltado (sin --vt-key)")

        # ── SecurityTrails ──
        if args.st_key:
            print("\n[SecurityTrails - Subdomain Enum + DNS History]")
            for domain in config.get("domains", []):
                print(f"  Dominio: {domain}")
                st = buscar_securitytrails(domain, args.st_key)
                todos[nombre]["securitytrails"][domain] = st
                subs = st.get("subdominios", [])
                print(f"    Subdominios: {len(subs)} (endpoint_count: {st.get('endpoint_count', '?')})")
                for s in subs[:25]:
                    print(f"    {s}")
                if len(subs) > 25:
                    print(f"    ... y {len(subs) - 25} más")
                # DNS history — highlight IP changes (staging/old infra)
                for rtype, records in st.get("dns_history", {}).items():
                    if records:
                        print(f"    DNS History ({rtype}):")
                        for rec in records[:5]:
                            ips = ", ".join(rec.get("values", [])[:5])
                            orgs = ", ".join(rec.get("organizations", [])[:2])
                            print(f"      {rec.get('first_seen', '?')} - {rec.get('last_seen', '?')}: {ips}" +
                                  (f" [{orgs}]" if orgs else ""))
                time.sleep(1)
        else:
            print("\n[SecurityTrails] Saltado (sin --st-key)")

        # ── RIPE Stat ──
        if not args.skip_ripestat:
            print("\n[RIPE Stat]")
            # Use first IP from each RIPE range
            for entry in todos[nombre]["ripe"][:5]:  # Limitar para no abusar
                inetnum = entry.get("inetnum", "")
                if not inetnum:
                    continue
                first_ip = inetnum.split(" - ")[0].strip()
                print(f"  Prefix: {first_ip}")
                stat = buscar_ripestat_prefix(first_ip)
                todos[nombre]["ripestat"][inetnum] = stat
                if stat.get("routing"):
                    print(f"    Routing: {stat['routing'].get('status')} (visibility: {stat['routing'].get('visibility')})")
                if stat.get("abuse_contacts"):
                    print(f"    Abuse: {', '.join(stat['abuse_contacts'])}")
                if stat.get("network_info", {}).get("asns"):
                    print(f"    ASNs: {', '.join(str(a) for a in stat['network_info']['asns'])}")

        # ── Reverse DNS ──
        if not args.skip_rdns:
            print("\n[Reverse DNS]")
            for entry in todos[nombre]["ripe"][:5]:  # Limitar
                inetnum = entry.get("inetnum", "")
                if not inetnum:
                    continue
                print(f"  Rango: {inetnum}")
                ptrs = reverse_dns(inetnum)
                todos[nombre]["reverse_dns"].extend(ptrs)
                for p in ptrs:
                    print(f"    {p['ip']} -> {p['ptr']}")

        # ── Port Scanning ──
        if not args.skip_portscan:
            print("\n[Port Scan]")
            # Scan first IP of each RIPE range + resolved A records from DNS
            scan_ips = set()
            for entry in todos[nombre]["ripe"][:5]:
                inetnum = entry.get("inetnum", "")
                if inetnum:
                    scan_ips.add(inetnum.split(" - ")[0].strip())
            for domain, dns_info in todos[nombre]["dns"].items():
                for ip in dns_info.get("A", [])[:3]:
                    scan_ips.add(ip)
            for ip in sorted(scan_ips):
                print(f"  IP: {ip}")
                metodo = None if args.scan_method == "auto" else args.scan_method
                abiertos = escanear_puertos(ip, timeout=args.scan_timeout, metodo_forzado=metodo)
                if abiertos:
                    todos[nombre]["portscan"][ip] = abiertos
                    for p in abiertos:
                        banner_str = f" | {p['banner'][:80]}" if p.get("banner") else ""
                        print(f"    {p['puerto']}/tcp OPEN ({p['metodo']}){banner_str}")
                else:
                    print(f"    Sin puertos abiertos (o filtrados)")

        # ── TLS SAN Analysis ──
        if not args.skip_san:
            print("\n[TLS Certificate SAN Analysis]")
            # Collect all known domains from previous steps
            dominios_conocidos = set()
            for c in todos[nombre]["crtsh"]:
                dominios_conocidos.add(c["subdomain"])
            for domain in config.get("domains", []):
                dominios_conocidos.add(domain)
            for domain, dns_info in todos[nombre]["dns"].items():
                dominios_conocidos.add(domain)
            for vt_data in todos[nombre]["virustotal"].values():
                for sub in vt_data.get("subdominios", []):
                    dominios_conocidos.add(sub.get("subdomain", ""))
            for st_data in todos[nombre]["securitytrails"].values():
                for sub in st_data.get("subdominios", []):
                    dominios_conocidos.add(sub)

            # Collect hosts to probe: IPs from DNS A records + RIPE ranges + discovered subdomains
            hosts_a_probar = []
            for domain, dns_info in todos[nombre]["dns"].items():
                for ip in dns_info.get("A", []):
                    hosts_a_probar.append(ip)
            for entry in todos[nombre]["ripe"][:5]:
                inetnum = entry.get("inetnum", "")
                if inetnum:
                    hosts_a_probar.append(inetnum.split(" - ")[0].strip())
            # Also probe discovered subdomains directly (TLS may reveal extra SANs)
            crtsh_subs = sorted(set(c["subdomain"] for c in todos[nombre]["crtsh"]
                                    if not c["subdomain"].startswith("*.")))
            hosts_a_probar.extend(crtsh_subs[:50])  # Limit to avoid excessive probing

            # Dedup
            hosts_a_probar = list(dict.fromkeys(hosts_a_probar))
            san_ports = [int(p.strip()) for p in args.san_ports.split(",")]

            print(f"  Probando {len(hosts_a_probar)} hosts en puertos {san_ports}")
            san_result = analizar_san(dominios_conocidos, hosts_a_probar,
                                      puertos_tls=san_ports, timeout=args.san_timeout)
            todos[nombre]["tls_san"] = san_result

            print(f"  Certificados obtenidos: {len(san_result['certs'])}")
            print(f"  SANs únicos totales: {san_result['total_unique_sans']}")

            if san_result["san_nuevos"]:
                print(f"  SANs NUEVOS (no en crt.sh/DNS/VT/ST): {len(san_result['san_nuevos'])}")
                for san in san_result["san_nuevos"][:30]:
                    print(f"    [NEW] {san}")
                if len(san_result["san_nuevos"]) > 30:
                    print(f"    ... y {len(san_result['san_nuevos']) - 30} más")

            if san_result["san_staging"]:
                print(f"  SANs con patrones STAGING/DEV/TEST: {len(san_result['san_staging'])}")
                for entry in san_result["san_staging"][:20]:
                    print(f"    [STAGING:{entry['pattern']}] {entry['san']}")

            if san_result["cohosted"]:
                print(f"  Dominios co-hosted (mismo SAN, múltiples hosts): {len(san_result['cohosted'])}")
                for san, hosts in list(san_result["cohosted"].items())[:10]:
                    print(f"    {san} -> {', '.join(hosts)}")

            if san_result["shared_certs"]:
                print(f"  Certs compartidos (mismos SANs): {len(san_result['shared_certs'])}")
                for sans, endpoints in list(san_result["shared_certs"].items())[:5]:
                    print(f"    [{', '.join(endpoints)}]")
                    print(f"      SANs: {sans[:120]}{'...' if len(sans) > 120 else ''}")
        else:
            print("\n[TLS SAN Analysis] Saltado (--skip-san)")

        # ── BGPView ──
        print("\n[BGPView]")
        for q in config["bgpview_queries"]:
            print(f"  Buscando ASNs: {q}")
            asns = buscar_bgpview_org(q)
            for asn_info in asns:
                print(f"  ASN{asn_info['asn']}: {asn_info['name']} ({asn_info['country']})")
                prefijos = buscar_bgpview_asn(asn_info["asn"])
                asn_info["prefijos"] = prefijos
                todos[nombre]["bgpview"].append(asn_info)
                for p in prefijos:
                    print(f"    {p['prefix']} - {p['description']}")
                time.sleep(0.5)

        # ── Shodan (opcional) ──
        if args.shodan_key:
            print("\n[Shodan]")
            for q in config["shodan_queries"]:
                print(f"  Query: {q}")
                hosts = buscar_shodan(q, args.shodan_key)
                todos[nombre]["shodan"].extend(hosts)
                for h in hosts:
                    print(f"    {h['ip']}:{h['puerto']} - {h.get('titulo', 'N/A')}")

    # Guardar resultados
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(todos, f, indent=2, ensure_ascii=False)
    print(f"\n[+] Resultados guardados en {args.output}")

    # Resumen
    print(f"\n{'='*60}")
    print("RESUMEN")
    print(f"{'='*60}")
    for nombre, data in todos.items():
        print(f"\n{nombre}:")
        print(f"  RIPE rangos:    {len(data['ripe'])}")
        print(f"  Subdominios CT: {len(set(c['subdomain'] for c in data['crtsh']))}")
        print(f"  DNS dominios:   {len(data['dns'])}")
        print(f"  RIPE Stat:      {len(data['ripestat'])} prefijos analizados")
        print(f"  Reverse DNS:    {len(data['reverse_dns'])} PTRs")
        wb_urls = sum(d.get("total_urls", 0) for d in data["wayback"].values())
        wb_rutas = sum(len(d.get("rutas_interesantes", [])) for d in data["wayback"].values())
        print(f"  Wayback URLs:   {wb_urls} ({wb_rutas} rutas interesantes)")
        ps_total = sum(len(v) for v in data["portscan"].values())
        print(f"  Puertos abiertos: {ps_total} en {len(data['portscan'])} IPs")
        vt_subs = sum(d.get("total_subdominios", 0) for d in data["virustotal"].values())
        vt_resol = sum(len(d.get("resoluciones_pasivas", [])) for d in data["virustotal"].values())
        print(f"  VT subdominios: {vt_subs} ({vt_resol} resoluciones pasivas)")
        st_subs = sum(len(d.get("subdominios", [])) for d in data["securitytrails"].values())
        st_hist = sum(len(recs) for d in data["securitytrails"].values() for recs in d.get("dns_history", {}).values())
        print(f"  ST subdominios: {st_subs} ({st_hist} DNS history records)")
        san = data.get("tls_san", {})
        print(f"  TLS SANs:       {san.get('total_unique_sans', 0)} únicos ({len(san.get('san_nuevos', []))} nuevos, {len(san.get('san_staging', []))} staging)")
        print(f"  BGPView ASNs:   {len(data['bgpview'])}")
        print(f"  Shodan hosts:   {len(data['shodan'])}")
