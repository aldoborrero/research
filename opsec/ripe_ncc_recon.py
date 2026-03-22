import requests
import json
import time
import socket
import ssl
import ipaddress
from collections import defaultdict
# ── RIPE DB helpers ──────────────────────────────────────────────────────────
_RIPE_DB_BASE = "https://rest.db.ripe.net"
_RIPE_DB_HEADERS = {"Accept": "application/json"}


def _ripe_parse_objects(data: dict, campos: set | None = None):
    """Parse RIPE DB JSON response into a list of dicts.

    If *campos* is None every attribute is kept.
    """
    resultados = []
    for obj in data.get("objects", {}).get("object", []):
        obj_type = obj.get("type", "")
        entry = defaultdict(list)
        for attr in obj.get("attributes", {}).get("attribute", []):
            name = attr["name"]
            if campos is None or name in campos:
                entry[name].append(attr["value"])
        final = {k: v[0] if len(v) == 1 else v for k, v in entry.items()}
        if final:
            final["_type"] = obj_type
            resultados.append(final)
    return resultados


# ── RIPE NCC DB — search ─────────────────────────────────────────────────────
def buscar_ripe(query: str, type_filter: str = "inetnum,inet6num,aut-num",
                flags: str = "no-referenced", delay: float = 1.0):
    """Busca objetos en la RIPE DB via full-text search.

    RIPE search API treats multi-word strings as multiple lookup keys.
    For org name searches, single keywords work best (e.g. "Indra" not
    "Indra Sistemas"). For exact ranges use IPs/AS numbers directly.
    """
    url = f"{_RIPE_DB_BASE}/search.json"
    params = {
        "query-string": query,
        "source": "RIPE",
    }
    if type_filter:
        params["type-filter"] = type_filter
    if flags:
        params["flags"] = flags

    try:
        r = requests.get(url, params=params, headers=_RIPE_DB_HEADERS, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] RIPE search error para '{query}': {e}")
        return []

    campos = {
        "inetnum", "inet6num", "aut-num", "netname", "descr", "org",
        "country", "status", "org-name", "org-type", "address", "mnt-by",
        "admin-c", "tech-c", "abuse-c", "route", "route6", "origin",
    }
    resultados = _ripe_parse_objects(r.json(), campos)
    time.sleep(delay)
    return resultados


# ── RIPE NCC DB — inverse lookups ────────────────────────────────────────────
def ripe_inverse_lookup(attr: str, value: str,
                        type_filter: str | None = None,
                        delay: float = 1.0):
    """Inverse lookup: find all objects referencing *value* via *attr*.

    Common attrs: org, admin-c, tech-c, mnt-by, abuse-c, origin, abuse-mailbox.
    """
    url = f"{_RIPE_DB_BASE}/search.json"
    params = {
        "query-string": value,
        "inverse-attribute": attr,
        "source": "RIPE",
        "flags": "no-referenced",
    }
    if type_filter:
        params["type-filter"] = type_filter

    try:
        r = requests.get(url, params=params, headers=_RIPE_DB_HEADERS, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] RIPE inverse ({attr}={value}) error: {e}")
        return []

    resultados = _ripe_parse_objects(r.json())
    time.sleep(delay)
    return resultados


def ripe_org_lookup(org_id: str, delay: float = 1.0):
    """Lookup a single organisation object by its handle (ORG-XXXX-RIPE)."""
    url = f"{_RIPE_DB_BASE}/ripe/organisation/{org_id}.json"
    try:
        r = requests.get(url, headers=_RIPE_DB_HEADERS, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] RIPE org lookup error para '{org_id}': {e}")
        return {}

    objs = _ripe_parse_objects(r.json())
    time.sleep(delay)
    return objs[0] if objs else {}


def ripe_abuse_contact(resource: str):
    """Get abuse contact for an IP/prefix/ASN via the RIPE DB abuse-contact endpoint."""
    url = f"{_RIPE_DB_BASE}/abuse-contact/{resource}.json"
    try:
        r = requests.get(url, headers=_RIPE_DB_HEADERS, timeout=10)
        r.raise_for_status()
        return r.json().get("abuse-contacts", {}).get("email")
    except requests.RequestException:
        return None


def ripe_find_org_resources(org_id: str, delay: float = 0.8):
    """Given an ORG-XXXX-RIPE handle, find all inetnums, inet6nums, and aut-nums.

    Returns dict with keys: inetnums, inet6nums, autnums (lists of parsed objects).
    """
    result = {"inetnums": [], "inet6nums": [], "autnums": []}

    for obj_type, key in [("inetnum", "inetnums"), ("inet6num", "inet6nums"), ("aut-num", "autnums")]:
        objs = ripe_inverse_lookup("org", org_id, type_filter=obj_type, delay=delay)
        result[key] = objs

    return result


def ripe_find_routes_for_asn(asn: str, delay: float = 0.8):
    """Find all route/route6 objects originated by an ASN.

    Uses inverse lookup on 'origin' attribute.
    """
    asn_str = asn if asn.upper().startswith("AS") else f"AS{asn}"
    routes4 = ripe_inverse_lookup("origin", asn_str, type_filter="route", delay=delay)
    routes6 = ripe_inverse_lookup("origin", asn_str, type_filter="route6", delay=delay)
    return {"route": routes4, "route6": routes6}


def ripe_find_by_maintainer(mntner: str, type_filter: str | None = None, delay: float = 0.8):
    """Find all objects managed by a maintainer (mnt-by inverse)."""
    return ripe_inverse_lookup("mnt-by", mntner, type_filter=type_filter, delay=delay)


def ripe_more_specific(prefix: str, delay: float = 1.0):
    """Find all more-specific (child) allocations within a prefix."""
    url = f"{_RIPE_DB_BASE}/search.json"
    params = {
        "query-string": prefix,
        "source": "RIPE",
        "flags": "all-more,no-referenced",
        "type-filter": "inetnum,inet6num",
    }
    try:
        r = requests.get(url, params=params, headers=_RIPE_DB_HEADERS, timeout=15)
        r.raise_for_status()
    except requests.RequestException as e:
        print(f"  [!] RIPE more-specific error para '{prefix}': {e}")
        return []

    resultados = _ripe_parse_objects(r.json())
    time.sleep(delay)
    return resultados
# ── RIPE STAT helpers ─────────────────────────────────────────────────────────
_RIPESTAT_BASE = "https://stat.ripe.net/data"


def _ripestat_get(endpoint: str, params: dict, label: str = ""):
    """Generic RIPE Stat data call. Returns the 'data' dict or {}."""
    url = f"{_RIPESTAT_BASE}/{endpoint}/data.json"
    try:
        r = requests.get(url, params=params, timeout=20)
        r.raise_for_status()
        return r.json().get("data", {})
    except requests.RequestException as e:
        print(f"  [!] RIPE Stat {label or endpoint} error: {e}")
        return {}


# ── RIPE STAT — prefix/IP queries ────────────────────────────────────────────
def ripestat_prefix_overview(prefix: str):
    """Prefix overview: origin ASNs, holder, related prefixes, announcement status."""
    data = _ripestat_get("prefix-overview", {"resource": prefix}, f"prefix-overview({prefix})")
    if not data:
        return {}
    return {
        "is_announced": data.get("announced"),
        "asns": [
            {"asn": a.get("asn"), "holder": a.get("holder")}
            for a in data.get("asns", [])
        ],
        "related_prefixes": [
            {"prefix": rp.get("prefix"), "origin_asn": rp.get("asn"), "relationship": rp.get("relationship")}
            for rp in data.get("related_prefixes", [])
        ],
        "resource": data.get("resource"),
        "block": data.get("block", {}).get("resource"),
    }


def ripestat_network_info(ip: str):
    """Network info: containing prefix + announcing ASN for a single IP."""
    data = _ripestat_get("network-info", {"resource": ip}, f"network-info({ip})")
    return {"asns": data.get("asns", []), "prefix": data.get("prefix")} if data else {}


def ripestat_address_space_hierarchy(prefix: str):
    """Address space hierarchy: parent (less-specific) and child (more-specific) allocations."""
    data = _ripestat_get("address-space-hierarchy", {"resource": prefix},
                         f"addr-hierarchy({prefix})")
    if not data:
        return {}
    exact = []
    for obj in data.get("exact", []):
        exact.append({
            "inetnum": obj.get("inetnum"),
            "netname": obj.get("netname"),
            "descr": obj.get("descr"),
            "org": obj.get("org"),
            "country": obj.get("country"),
            "status": obj.get("status"),
        })
    more_specific = [
        {"inetnum": o.get("inetnum"), "netname": o.get("netname")}
        for o in data.get("more_specific", [])
    ]
    less_specific = [
        {"inetnum": o.get("inetnum"), "netname": o.get("netname")}
        for o in data.get("less_specific", [])
    ]
    return {
        "rir": data.get("rir"),
        "exact": exact,
        "more_specific": more_specific,
        "less_specific": less_specific,
    }


def ripestat_routing_status(resource: str):
    """Routing status: first/last seen, visibility, origin ASes."""
    data = _ripestat_get("routing-status", {"resource": resource},
                         f"routing-status({resource})")
    if not data:
        return {}
    return {
        "status": data.get("status"),
        "first_seen": data.get("first_seen"),
        "last_seen": data.get("last_seen"),
        "visibility_v4": data.get("visibility", {}).get("v4", {}).get("total_peers"),
        "visibility_v6": data.get("visibility", {}).get("v6", {}).get("total_peers"),
        "announced_space": data.get("announced_space"),
        "observed_neighbours": data.get("observed_neighbours"),
    }


def ripestat_geolocation(prefix: str):
    """Geolocation via MaxMind GeoLite."""
    data = _ripestat_get("maxmind-geo-lite", {"resource": prefix}, f"geo({prefix})")
    if not data:
        return []
    return [
        {
            "prefix": loc.get("resource"),
            "country": loc.get("locations", [{}])[0].get("country") if loc.get("locations") else None,
            "city": loc.get("locations", [{}])[0].get("city") if loc.get("locations") else None,
            "latitude": loc.get("locations", [{}])[0].get("latitude") if loc.get("locations") else None,
            "longitude": loc.get("locations", [{}])[0].get("longitude") if loc.get("locations") else None,
        }
        for loc in data.get("located_resources", [])
    ]


def ripestat_abuse_contacts(resource: str):
    """Abuse contact finder."""
    data = _ripestat_get("abuse-contact-finder", {"resource": resource},
                         f"abuse({resource})")
    return {
        "abuse_contacts": data.get("abuse_contacts", []),
        "authoritative_rir": data.get("authoritative_rir"),
    } if data else {}


def ripestat_reverse_dns(prefix: str):
    """Reverse DNS delegations for a prefix."""
    data = _ripestat_get("reverse-dns", {"resource": prefix}, f"rdns({prefix})")
    if not data:
        return []
    delegations = []
    for d in data.get("delegations", []):
        delegations.append({
            "key": d.get("key"),
            "value": d.get("value"),
            "nservers": d.get("nservers", []),
        })
    return delegations


def ripestat_dns_chain(hostname: str):
    """DNS resolution chain for a hostname."""
    data = _ripestat_get("dns-chain", {"resource": hostname}, f"dns-chain({hostname})")
    if not data:
        return {}
    return {
        "forward_nodes": data.get("forward_nodes", []),
        "reverse_nodes": data.get("reverse_nodes", []),
        "nameservers": data.get("nameservers", []),
    }


def ripestat_rpki_validation(asn: str, prefix: str):
    """RPKI validation for an ASN+prefix pair."""
    data = _ripestat_get("rpki-validation", {"resource": asn, "prefix": prefix},
                         f"rpki({asn},{prefix})")
    if not data:
        return {}
    return {
        "status": data.get("status"),
        "description": data.get("description"),
        "validating_roas": data.get("validating_roas", []),
    }


def ripestat_whois(resource: str):
    """Full WHOIS lookup across all RIRs."""
    data = _ripestat_get("whois", {"resource": resource}, f"whois({resource})")
    if not data:
        return {}
    records = []
    for rec in data.get("records", []):
        record_attrs = {}
        for attr in rec:
            record_attrs[attr.get("key", "")] = attr.get("value", "")
        if record_attrs:
            records.append(record_attrs)
    return {
        "records": records,
        "irr_records": data.get("irr_records", []),
        "authorities": data.get("authorities", []),
    }


def ripestat_historical_whois(resource: str):
    """Historical WHOIS: version count and change history."""
    data = _ripestat_get("historical-whois", {"resource": resource},
                         f"hist-whois({resource})")
    if not data:
        return {}
    versions = []
    for obj in data.get("objects", []):
        for v in obj.get("versions", []):
            versions.append({
                "version": v.get("version"),
                "from": v.get("from"),
                "to": v.get("to"),
            })
    return {
        "num_versions": data.get("num_versions"),
        "versions": versions,
    }


# ── RIPE STAT — ASN queries ─────────────────────────────────────────────────
def ripestat_as_overview(asn: str):
    """AS overview: holder name, announcement status, type."""
    data = _ripestat_get("as-overview", {"resource": asn}, f"as-overview({asn})")
    if not data:
        return {}
    return {
        "holder": data.get("holder"),
        "announced": data.get("announced"),
        "type": data.get("type"),
        "block": data.get("block", {}).get("resource"),
    }


def ripestat_announced_prefixes(asn: str):
    """All prefixes announced by an ASN, with visibility timelines."""
    data = _ripestat_get("announced-prefixes", {"resource": asn},
                         f"announced-prefixes({asn})")
    if not data:
        return []
    return [
        {
            "prefix": p.get("prefix"),
            "timelines": [
                {"starttime": t.get("starttime"), "endtime": t.get("endtime")}
                for t in p.get("timelines", [])
            ],
        }
        for p in data.get("prefixes", [])
    ]


def ripestat_asn_neighbours(asn: str):
    """ASN neighbours: peering/transit relationships."""
    data = _ripestat_get("asn-neighbours", {"resource": asn},
                         f"asn-neighbours({asn})")
    if not data:
        return {}
    neighbours = []
    for n in data.get("neighbours", []):
        neighbours.append({
            "asn": n.get("asn"),
            "type": n.get("type"),  # left/right/uncertain
            "power": n.get("power"),
            "v4_peers": n.get("v4_peers"),
            "v6_peers": n.get("v6_peers"),
        })
    return {
        "neighbour_count": data.get("neighbour_counts", {}),
        "neighbours": neighbours,
    }


def ripestat_asn_neighbours_history(asn: str):
    """Historical ASN peering/transit neighbours over time."""
    data = _ripestat_get("asn-neighbours-history", {"resource": asn},
                         f"asn-neighbours-history({asn})")
    if not data:
        return []
    return [
        {
            "neighbour": n.get("neighbour"),
            "type": n.get("type"),
            "timelines": [
                {"starttime": t.get("starttime"), "endtime": t.get("endtime")}
                for t in n.get("timelines", [])
            ],
        }
        for n in data.get("neighbours", [])
    ]


def ripestat_looking_glass(prefix: str):
    """Looking glass: per-RRC collector routing data."""
    data = _ripestat_get("looking-glass", {"resource": prefix},
                         f"looking-glass({prefix})")
    if not data:
        return []
    rrcs = []
    for rrc in data.get("rrcs", []):
        peers = []
        for p in rrc.get("peers", []):
            peers.append({
                "asn_origin": p.get("asn_origin"),
                "as_path": p.get("as_path"),
                "community": p.get("community"),
                "next_hop": p.get("next_hop"),
                "peer": p.get("peer"),
            })
        rrcs.append({
            "rrc": rrc.get("rrc"),
            "location": rrc.get("location"),
            "peers": peers,
        })
    return rrcs


def ripestat_routing_history(resource: str, starttime: str | None = None,
                              endtime: str | None = None):
    """Routing history: BGP routing changes over time."""
    params = {"resource": resource, "min_peers": 5}
    if starttime:
        params["starttime"] = starttime
    if endtime:
        params["endtime"] = endtime
    data = _ripestat_get("routing-history", params, f"routing-history({resource})")
    if not data:
        return []
    entries = []
    for route in data.get("by_origin", []):
        origin = route.get("origin")
        prefixes = []
        for p in route.get("prefixes", []):
            prefixes.append({
                "prefix": p.get("prefix"),
                "timelines": [
                    {"starttime": t.get("starttime"), "endtime": t.get("endtime")}
                    for t in p.get("timelines", [])
                ],
            })
        entries.append({"origin": origin, "prefixes": prefixes})
    return entries


def ripestat_prefix_routing_consistency(asn: str):
    """Prefix routing consistency: whether announced prefixes match WHOIS."""
    data = _ripestat_get("prefix-routing-consistency", {"resource": asn},
                         f"prefix-routing-consistency({asn})")
    if not data:
        return []
    return [
        {
            "prefix": r.get("prefix"),
            "origin": r.get("origin"),
            "in_whois": r.get("in_whois"),
            "in_bgp": r.get("in_bgp"),
            "irr_sources": r.get("irr_sources"),
        }
        for r in data.get("routes", [])
    ]


def ripestat_geo_by_asn(asn: str):
    """Geographic distribution of prefixes announced by an ASN."""
    data = _ripestat_get("maxmind-geo-lite-announced-by-as", {"resource": asn},
                         f"geo-by-asn({asn})")
    if not data:
        return []
    return [
        {
            "prefix": loc.get("resource"),
            "country": loc.get("locations", [{}])[0].get("country") if loc.get("locations") else None,
            "city": loc.get("locations", [{}])[0].get("city") if loc.get("locations") else None,
        }
        for loc in data.get("located_resources", [])
    ]


# ── RIPE STAT — country queries ──────────────────────────────────────────────
def ripestat_country_resources(country_code: str):
    """All ASNs, IPv4 ranges, and IPv6 prefixes allocated to a country."""
    data = _ripestat_get("country-resource-list",
                         {"resource": country_code, "v4_format": "prefix"},
                         f"country-resources({country_code})")
    if not data:
        return {}
    resources = data.get("resources", {})
    return {
        "asn": resources.get("asn", []),
        "ipv4": resources.get("ipv4", []),
        "ipv6": resources.get("ipv6", []),
    }


# ── RIPE STAT — combined prefix analysis (replaces old buscar_ripestat_prefix)
def buscar_ripestat_prefix(prefix: str, include_hierarchy: bool = False,
                            include_looking_glass: bool = False):
    """Comprehensive RIPE Stat analysis for a prefix.

    Always queries: routing-status, geolocation, abuse-contacts, network-info,
    prefix-overview, RPKI validation.
    Optionally: address-space-hierarchy, looking-glass.
    """
    resultados = {}

    resultados["routing"] = ripestat_routing_status(prefix)
    time.sleep(0.3)

    resultados["geolocation"] = ripestat_geolocation(prefix)
    time.sleep(0.3)

    abuse = ripestat_abuse_contacts(prefix)
    resultados["abuse_contacts"] = abuse.get("abuse_contacts", [])
    resultados["authoritative_rir"] = abuse.get("authoritative_rir")
    time.sleep(0.3)

    resultados["network_info"] = ripestat_network_info(prefix)
    time.sleep(0.3)

    resultados["prefix_overview"] = ripestat_prefix_overview(prefix)
    time.sleep(0.3)

    # RPKI validation if we know the origin ASN
    asns = resultados["network_info"].get("asns", [])
    if asns:
        containing_prefix = resultados["network_info"].get("prefix", prefix)
        rpki = ripestat_rpki_validation(f"AS{asns[0]}", containing_prefix)
        resultados["rpki"] = rpki
        time.sleep(0.3)

    if include_hierarchy:
        resultados["hierarchy"] = ripestat_address_space_hierarchy(prefix)
        time.sleep(0.3)

    if include_looking_glass:
        resultados["looking_glass"] = ripestat_looking_glass(prefix)
        time.sleep(0.3)

    return resultados


# ── RIPE STAT — combined ASN analysis ────────────────────────────────────────
def buscar_ripestat_asn(asn: str, include_neighbours_history: bool = False,
                         include_routing_consistency: bool = False):
    """Comprehensive RIPE Stat analysis for an ASN.

    Always queries: as-overview, announced-prefixes, asn-neighbours, geo-by-asn.
    Optionally: asn-neighbours-history, prefix-routing-consistency.
    """
    asn_str = asn if asn.upper().startswith("AS") else f"AS{asn}"
    resultados = {}

    resultados["overview"] = ripestat_as_overview(asn_str)
    time.sleep(0.3)

    resultados["announced_prefixes"] = ripestat_announced_prefixes(asn_str)
    time.sleep(0.3)

    resultados["neighbours"] = ripestat_asn_neighbours(asn_str)
    time.sleep(0.3)

    resultados["geo_distribution"] = ripestat_geo_by_asn(asn_str)
    time.sleep(0.3)

    if include_neighbours_history:
        resultados["neighbours_history"] = ripestat_asn_neighbours_history(asn_str)
        time.sleep(0.3)

    if include_routing_consistency:
        resultados["routing_consistency"] = ripestat_prefix_routing_consistency(asn_str)
        time.sleep(0.3)

    return resultados


# ── ORG → ASN → PREFIX chaining ──────────────────────────────────────────────
def ripe_chain_org_to_prefixes(org_queries: list, delay: float = 0.8):
    """Full chain: org name → org handle → inetnums + ASNs → routes + announced prefixes.

    Takes a list of org search queries (e.g. ["AEAT", "Agencia Tributaria"]).
    Returns a comprehensive mapping of the organization's IP infrastructure.
    """
    chain = {
        "org_handles": [],
        "inetnums": [],
        "inet6nums": [],
        "autnums": [],
        "routes": [],
        "announced_prefixes": [],
        "maintainers": set(),
    }

    # Step 1: Search for organisation objects
    seen_orgs = set()
    for query in org_queries:
        orgs = buscar_ripe(query, type_filter="organisation", flags="no-referenced", delay=delay)
        for org in orgs:
            org_id = org.get("org") or org.get("organisation")
            if org_id and org_id not in seen_orgs:
                seen_orgs.add(org_id)
                chain["org_handles"].append(org)

    # Also extract org handles from inetnum/aut-num results
    general = buscar_ripe(org_queries[0] if org_queries else "", delay=delay)
    for obj in general:
        org_id = obj.get("org")
        if org_id and org_id not in seen_orgs:
            seen_orgs.add(org_id)
            # Look up the org object
            org_detail = ripe_org_lookup(org_id, delay=delay)
            if org_detail:
                chain["org_handles"].append(org_detail)

    # Step 2: For each org, find all resources via inverse lookup
    seen_inetnums = set()
    seen_autnums = set()
    for org_id in seen_orgs:
        resources = ripe_find_org_resources(org_id, delay=delay)
        for inet in resources["inetnums"]:
            key = inet.get("inetnum", "")
            if key and key not in seen_inetnums:
                seen_inetnums.add(key)
                chain["inetnums"].append(inet)
        for inet6 in resources["inet6nums"]:
            key = inet6.get("inet6num", "")
            if key not in seen_inetnums:
                seen_inetnums.add(key)
                chain["inet6nums"].append(inet6)
        for autnum in resources["autnums"]:
            asn = autnum.get("aut-num", "")
            if asn and asn not in seen_autnums:
                seen_autnums.add(asn)
                chain["autnums"].append(autnum)
            # Collect maintainers for later
            mnt = autnum.get("mnt-by")
            if mnt:
                if isinstance(mnt, list):
                    chain["maintainers"].update(mnt)
                else:
                    chain["maintainers"].add(mnt)

    # Step 3: For each ASN, find route objects and announced prefixes
    for asn in seen_autnums:
        routes = ripe_find_routes_for_asn(asn, delay=delay)
        chain["routes"].extend(routes.get("route", []))
        chain["routes"].extend(routes.get("route6", []))

        announced = ripestat_announced_prefixes(asn)
        for p in announced:
            p["origin_asn"] = asn
        chain["announced_prefixes"].extend(announced)
        time.sleep(0.3)

    # Convert set to list for JSON serialization
    chain["maintainers"] = sorted(chain["maintainers"])

    return chain
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
        "ripe_org_queries": ["DGT", "Direccion General de Trafico"],
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
        "ripe_org_queries": ["Indra", "Indra Sistemas"],
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
        "ripe_org_queries": ["AEAT", "Agencia Tributaria"],
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
        description="OSINT reconnaissance: RIPE (DB+Stat), BGPView, crt.sh, DNS, VirusTotal, SecurityTrails, Shodan"
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
    parser.add_argument("--skip-ripe-chain", action="store_true", help="Saltar RIPE org→ASN→prefix chaining")
    parser.add_argument("--ripe-hierarchy", action="store_true", help="Include RIPE address-space-hierarchy")
    parser.add_argument("--ripe-looking-glass", action="store_true", help="Include RIPE looking glass data")
    parser.add_argument("--ripe-asn-history", action="store_true", help="Include ASN neighbour history")
    parser.add_argument("--ripe-routing-consistency", action="store_true", help="Include prefix routing consistency")
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
            "ripe": [], "ripe_chain": {}, "bgpview": [], "shodan": [],
            "crtsh": [], "dns": {}, "ripestat_prefixes": {}, "ripestat_asns": {},
            "reverse_dns": [], "wayback": {}, "portscan": {},
            "virustotal": {}, "securitytrails": {}, "tls_san": {},
        }

        # ── RIPE NCC DB — basic search ──
        print("\n[RIPE NCC — DB Search]")
        for q in config["ripe_queries"]:
            print(f"  Query: {q}")
            res = buscar_ripe(q)
            todos[nombre]["ripe"].extend(res)
            for r in res:
                print(f"    [{r.get('_type', '?')}] {r.get('inetnum', r.get('inet6num', r.get('aut-num', '?')))} - {r.get('netname', '?')} ({r.get('country', '?')})")

        # ── RIPE NCC DB — org→ASN→prefix chain ──
        if not args.skip_ripe_chain:
            print("\n[RIPE NCC — Org→ASN→Prefix Chain]")
            org_queries = config.get("ripe_org_queries", config["ripe_queries"])
            chain = ripe_chain_org_to_prefixes(org_queries)
            todos[nombre]["ripe_chain"] = chain

            if chain["org_handles"]:
                print(f"  Org handles found: {len(chain['org_handles'])}")
                for org in chain["org_handles"]:
                    org_id = org.get("org") or org.get("organisation") or "?"
                    org_name = org.get("org-name", org.get("descr", "?"))
                    print(f"    {org_id}: {org_name}")

            if chain["inetnums"]:
                print(f"  IPv4 ranges (inverse org lookup): {len(chain['inetnums'])}")
                for inet in chain["inetnums"][:20]:
                    print(f"    {inet.get('inetnum', '?')} - {inet.get('netname', '?')} ({inet.get('country', '?')})")
                if len(chain["inetnums"]) > 20:
                    print(f"    ... y {len(chain['inetnums']) - 20} más")

            if chain["inet6nums"]:
                print(f"  IPv6 ranges: {len(chain['inet6nums'])}")
                for inet6 in chain["inet6nums"][:10]:
                    print(f"    {inet6.get('inet6num', '?')} - {inet6.get('netname', '?')}")

            if chain["autnums"]:
                print(f"  Autonomous Systems: {len(chain['autnums'])}")
                for asn in chain["autnums"]:
                    print(f"    {asn.get('aut-num', '?')} - {asn.get('descr', asn.get('org-name', '?'))}")

            if chain["routes"]:
                print(f"  Route objects: {len(chain['routes'])}")
                for route in chain["routes"][:20]:
                    print(f"    {route.get('route', route.get('route6', '?'))} origin {route.get('origin', '?')}")
                if len(chain["routes"]) > 20:
                    print(f"    ... y {len(chain['routes']) - 20} más")

            if chain["announced_prefixes"]:
                print(f"  Announced prefixes (BGP): {len(chain['announced_prefixes'])}")
                for p in chain["announced_prefixes"][:20]:
                    print(f"    {p.get('prefix', '?')} (origin {p.get('origin_asn', '?')})")
                if len(chain["announced_prefixes"]) > 20:
                    print(f"    ... y {len(chain['announced_prefixes']) - 20} más")

            if chain["maintainers"]:
                print(f"  Maintainers: {', '.join(chain['maintainers'])}")

            # Merge chain results into the basic ripe list for downstream use
            for inet in chain["inetnums"]:
                if inet not in todos[nombre]["ripe"]:
                    todos[nombre]["ripe"].append(inet)
        else:
            print("\n[RIPE NCC — Org Chain] Saltado (--skip-ripe-chain)")

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

        # ── RIPE Stat — prefix analysis ──
        if not args.skip_ripestat:
            print("\n[RIPE Stat — Prefix Analysis]")
            # Use first IP from each RIPE range
            seen_prefixes = set()
            for entry in todos[nombre]["ripe"][:10]:
                inetnum = entry.get("inetnum", "")
                if not inetnum:
                    continue
                first_ip = inetnum.split(" - ")[0].strip()
                if first_ip in seen_prefixes:
                    continue
                seen_prefixes.add(first_ip)
                print(f"  Prefix: {first_ip}")
                stat = buscar_ripestat_prefix(
                    first_ip,
                    include_hierarchy=args.ripe_hierarchy,
                    include_looking_glass=args.ripe_looking_glass,
                )
                todos[nombre]["ripestat_prefixes"][inetnum] = stat
                if stat.get("routing"):
                    vis = stat["routing"].get("visibility_v4") or stat["routing"].get("visibility_v6")
                    print(f"    Routing: {stat['routing'].get('status')} (visibility: {vis})")
                if stat.get("abuse_contacts"):
                    print(f"    Abuse: {', '.join(stat['abuse_contacts'])}")
                ni = stat.get("network_info", {})
                if ni.get("asns"):
                    print(f"    ASNs: {', '.join(str(a) for a in ni['asns'])}")
                    print(f"    Containing prefix: {ni.get('prefix', '?')}")
                po = stat.get("prefix_overview", {})
                if po.get("asns"):
                    for a in po["asns"]:
                        print(f"    Origin: AS{a.get('asn')} ({a.get('holder', '?')})")
                rpki = stat.get("rpki", {})
                if rpki.get("status"):
                    print(f"    RPKI: {rpki['status']}")
                if stat.get("hierarchy"):
                    h = stat["hierarchy"]
                    print(f"    Hierarchy: {len(h.get('less_specific', []))} parents, {len(h.get('more_specific', []))} children")

            # ── RIPE Stat — ASN analysis ──
            print("\n[RIPE Stat — ASN Analysis]")
            chain = todos[nombre].get("ripe_chain", {})
            discovered_asns = set()

            # Collect ASNs from chain + ripestat prefix results
            for autnum in chain.get("autnums", []):
                asn = autnum.get("aut-num", "")
                if asn:
                    discovered_asns.add(asn)
            for stat in todos[nombre]["ripestat_prefixes"].values():
                for a in stat.get("network_info", {}).get("asns", []):
                    discovered_asns.add(f"AS{a}" if not str(a).upper().startswith("AS") else str(a))

            for asn in sorted(discovered_asns)[:10]:
                print(f"  ASN: {asn}")
                asn_stat = buscar_ripestat_asn(
                    asn,
                    include_neighbours_history=args.ripe_asn_history,
                    include_routing_consistency=args.ripe_routing_consistency,
                )
                todos[nombre]["ripestat_asns"][asn] = asn_stat
                ov = asn_stat.get("overview", {})
                if ov:
                    print(f"    Holder: {ov.get('holder', '?')} (announced: {ov.get('announced')})")
                prefixes = asn_stat.get("announced_prefixes", [])
                if prefixes:
                    print(f"    Announced prefixes: {len(prefixes)}")
                    for p in prefixes[:10]:
                        print(f"      {p.get('prefix', '?')}")
                    if len(prefixes) > 10:
                        print(f"      ... y {len(prefixes) - 10} más")
                nb = asn_stat.get("neighbours", {})
                if nb.get("neighbours"):
                    print(f"    Neighbours: {len(nb['neighbours'])} ({nb.get('neighbour_count', {})})")
                    for n in nb["neighbours"][:10]:
                        print(f"      AS{n.get('asn')} ({n.get('type', '?')}, power={n.get('power', '?')})")
                    if len(nb["neighbours"]) > 10:
                        print(f"      ... y {len(nb['neighbours']) - 10} más")
                geo = asn_stat.get("geo_distribution", [])
                if geo:
                    countries = set(g.get("country") for g in geo if g.get("country"))
                    print(f"    Geo distribution: {len(geo)} prefixes across {len(countries)} countries: {', '.join(sorted(countries))}")
                rc = asn_stat.get("routing_consistency", [])
                if rc:
                    in_both = sum(1 for r in rc if r.get("in_whois") and r.get("in_bgp"))
                    bgp_only = sum(1 for r in rc if r.get("in_bgp") and not r.get("in_whois"))
                    print(f"    Routing consistency: {in_both} registered+announced, {bgp_only} BGP-only")

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
        chain = data.get("ripe_chain", {})
        if chain:
            print(f"  RIPE chain:     {len(chain.get('org_handles', []))} orgs, "
                  f"{len(chain.get('inetnums', []))} inetnums, "
                  f"{len(chain.get('autnums', []))} ASNs, "
                  f"{len(chain.get('routes', []))} routes, "
                  f"{len(chain.get('announced_prefixes', []))} announced")
        print(f"  Subdominios CT: {len(set(c['subdomain'] for c in data['crtsh']))}")
        print(f"  DNS dominios:   {len(data['dns'])}")
        print(f"  RIPE Stat pfx:  {len(data.get('ripestat_prefixes', {}))} prefijos analizados")
        print(f"  RIPE Stat ASN:  {len(data.get('ripestat_asns', {}))} ASNs analizados")
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
