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
    args = parser.parse_args()

    targets = TARGETS if args.target == "all" else {args.target: TARGETS[args.target]}
    todos = {}

    for nombre, config in targets.items():
        print(f"\n{'='*60}")
        print(f"TARGET: {nombre}")
        print(f"{'='*60}")

        todos[nombre] = {
            "ripe": [], "bgpview": [], "shodan": [],
            "crtsh": [], "dns": {}, "ripestat": {}, "reverse_dns": [],
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
        print(f"  BGPView ASNs:   {len(data['bgpview'])}")
        print(f"  Shodan hosts:   {len(data['shodan'])}")
