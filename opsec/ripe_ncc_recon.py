import requests
import json
import time
import socket
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
        description="OSINT reconnaissance: RIPE, BGPView, crt.sh, DNS, RIPE Stat, Shodan"
    )
    parser.add_argument("--shodan-key", default=None)
    parser.add_argument("--target", choices=list(TARGETS.keys()) + ["all"], default="all")
    parser.add_argument("--output", default="resultados.json")
    parser.add_argument("--skip-rdns", action="store_true", help="Saltar reverse DNS (lento)")
    parser.add_argument("--skip-ripestat", action="store_true", help="Saltar RIPE Stat (rate limited)")
    parser.add_argument("--skip-wayback", action="store_true", help="Saltar Wayback Machine")
    parser.add_argument("--skip-portscan", action="store_true", help="Saltar port scanning")
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
        print(f"  BGPView ASNs:   {len(data['bgpview'])}")
        print(f"  Shodan hosts:   {len(data['shodan'])}")
