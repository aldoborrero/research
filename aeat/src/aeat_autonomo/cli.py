"""CLI entrypoint for AEAT autónomo tax automation.

Usage:
    aeat generate 303 --quarter 1T --year 2026
    aeat generate 130 --quarter 1T --year 2026
    aeat submit 303 --quarter 1T --year 2026 [--dry-run]
    aeat quipu-totals --quarter 1T --year 2026
"""

from __future__ import annotations

import json
import sys
from decimal import Decimal
from pathlib import Path

import click

from .config import CertificateConfig, QuipuConfig
from .modelo130 import Modelo130Data, generate_130_boe
from .modelo303 import Modelo303Data, generate_303_boe
from .quipu import QuipuClient


def _load_config(config_path: Path) -> dict:
    """Load configuration from a JSON file."""
    if not config_path.exists():
        click.echo(f"Config file not found: {config_path}", err=True)
        click.echo("Create one with: aeat init", err=True)
        sys.exit(1)
    return json.loads(config_path.read_text())


@click.group()
@click.option(
    "--config",
    "config_path",
    type=click.Path(path_type=Path),
    default=Path("aeat-config.json"),
    envvar="AEAT_CONFIG",
    help="Path to config file.",
)
@click.pass_context
def main(ctx: click.Context, config_path: Path) -> None:
    """AEAT tax automation for autónomos."""
    ctx.ensure_object(dict)
    ctx.obj["config_path"] = config_path


@main.command()
@click.pass_context
def init(ctx: click.Context) -> None:
    """Create a sample configuration file."""
    config_path: Path = ctx.obj["config_path"]
    if config_path.exists():
        click.confirm(f"{config_path} already exists. Overwrite?", abort=True)

    sample = {
        "declarant": {
            "nif": "12345678A",
            "apellidos": "GARCIA LOPEZ",
            "nombre": "JUAN",
        },
        "certificate": {
            "pfx_path": "./certificado.p12",
            "password": "YOUR_PASSWORD_HERE",
        },
        "quipu": {
            "api_key": "YOUR_QUIPU_APP_ID",
            "api_secret": "YOUR_QUIPU_APP_SECRET",
        },
        "iban": "ES00 0000 0000 0000 0000 0000",
        "testing": True,
    }
    config_path.write_text(json.dumps(sample, indent=2, ensure_ascii=False))
    click.echo(f"Created sample config at {config_path}")
    click.echo("Edit it with your real credentials before use.")


@main.command("quipu-totals")
@click.option("--year", required=True, type=int, help="Fiscal year.")
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.pass_context
def quipu_totals(ctx: click.Context, year: int, quarter: str) -> None:
    """Fetch quarterly totals from Quipu."""
    config = _load_config(ctx.obj["config_path"])
    quipu_cfg = QuipuConfig(
        api_key=config["quipu"]["api_key"],
        api_secret=config["quipu"]["api_secret"],
    )

    q_num = int(quarter[0])
    with QuipuClient(quipu_cfg) as client:
        totals = client.get_quarterly_totals(year, q_num)

    click.echo(f"Quipu quarterly totals — {year} {quarter}")
    click.echo(f"  Income (base):      {totals.total_income_gross:>12.2f} €")
    click.echo(f"  VAT collected:      {totals.total_vat_collected:>12.2f} €")
    click.echo(f"  Expenses (base):    {totals.total_expenses_gross:>12.2f} €")
    click.echo(f"  VAT deductible:     {totals.total_vat_deductible:>12.2f} €")
    click.echo(f"  ---")
    click.echo(f"  Net VAT (303):      {totals.net_vat:>12.2f} €")
    click.echo(f"  Net income (130):   {totals.net_income:>12.2f} €")


@main.group()
def generate() -> None:
    """Generate BOE files for AEAT models."""


@generate.command("303")
@click.option("--year", required=True, type=int)
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.option("--from-quipu", is_flag=True, help="Fetch data from Quipu automatically.")
@click.option("--output", "-o", type=click.Path(path_type=Path), default=None)
@click.pass_context
def generate_303(
    ctx: click.Context,
    year: int,
    quarter: str,
    from_quipu: bool,
    output: Path | None,
) -> None:
    """Generate Modelo 303 (quarterly VAT)."""
    config = _load_config(ctx.obj["config_path"])
    declarant = config["declarant"]

    if from_quipu:
        quipu_cfg = QuipuConfig(
            api_key=config["quipu"]["api_key"],
            api_secret=config["quipu"]["api_secret"],
        )
        q_num = int(quarter[0])
        with QuipuClient(quipu_cfg) as client:
            totals = client.get_quarterly_totals(year, q_num)

        data = Modelo303Data(
            nif=declarant["nif"],
            name=declarant.get("apellidos", "") + " " + declarant.get("nombre", ""),
            exercise=year,
            period=quarter,
            base_21=totals.total_income_gross,
            vat_21=totals.total_vat_collected,
            base_deductible_domestic=totals.total_expenses_gross,
            vat_deductible_domestic=totals.total_vat_deductible,
            cuenta_iban=config.get("iban", ""),
        )
    else:
        # Interactive input
        click.echo("Enter amounts for Modelo 303:")
        data = Modelo303Data(
            nif=declarant["nif"],
            name=declarant.get("apellidos", "") + " " + declarant.get("nombre", ""),
            exercise=year,
            period=quarter,
            base_21=Decimal(click.prompt("Base imponible 21%", default="0")),
            vat_21=Decimal(click.prompt("Cuota IVA 21%", default="0")),
            base_10=Decimal(click.prompt("Base imponible 10%", default="0")),
            vat_10=Decimal(click.prompt("Cuota IVA 10%", default="0")),
            base_deductible_domestic=Decimal(
                click.prompt("Base IVA deducible (gastos)", default="0")
            ),
            vat_deductible_domestic=Decimal(
                click.prompt("Cuota IVA deducible", default="0")
            ),
            cuenta_iban=config.get("iban", ""),
        )

    # Determine declaration type
    if data.resultado > 0:
        data.tipo_declaracion = "I"  # Ingreso
    elif data.resultado < 0:
        if quarter == "4T":
            data.tipo_declaracion = "D"  # Devolución (only in Q4)
        else:
            data.tipo_declaracion = "C"  # Compensar
    else:
        data.tipo_declaracion = "N"  # Negativa

    boe = generate_303_boe(data)

    if output:
        output.write_text(boe)
        click.echo(f"Written to {output}")
    else:
        click.echo(boe)

    click.echo(f"\n--- Summary ---")
    click.echo(f"IVA repercutido:  {data.total_vat_collected:>10.2f} €")
    click.echo(f"IVA deducible:    {data.total_vat_deductible:>10.2f} €")
    click.echo(f"Resultado (69):   {data.resultado:>10.2f} €")
    click.echo(f"Tipo declaración: {data.tipo_declaracion}")


@generate.command("130")
@click.option("--year", required=True, type=int)
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.option("--from-quipu", is_flag=True, help="Fetch data from Quipu automatically.")
@click.option(
    "--prev-payments",
    type=str,
    default="0",
    help="Previous quarters' advance payments this year.",
)
@click.option("--output", "-o", type=click.Path(path_type=Path), default=None)
@click.pass_context
def generate_130(
    ctx: click.Context,
    year: int,
    quarter: str,
    from_quipu: bool,
    prev_payments: str,
    output: Path | None,
) -> None:
    """Generate Modelo 130 (quarterly IRPF advance)."""
    config = _load_config(ctx.obj["config_path"])
    declarant = config["declarant"]

    if from_quipu:
        quipu_cfg = QuipuConfig(
            api_key=config["quipu"]["api_key"],
            api_secret=config["quipu"]["api_secret"],
        )
        # For 130, we need ACCUMULATED income from Q1 through current quarter
        q_num = int(quarter[0])
        total_income = Decimal("0")
        total_expenses = Decimal("0")

        with QuipuClient(quipu_cfg) as client:
            for q in range(1, q_num + 1):
                totals = client.get_quarterly_totals(year, q)
                total_income += totals.total_income_gross
                total_expenses += totals.total_expenses_gross

        rendimiento_neto = total_income - total_expenses

        data = Modelo130Data(
            nif=declarant["nif"],
            apellidos=declarant.get("apellidos", declarant.get("name", "")),
            nombre=declarant.get("nombre", ""),
            exercise=year,
            period=quarter,
            ingresos=total_income,
            gastos=total_expenses,
            pagos_anteriores=Decimal(prev_payments),
            cuenta_iban=config.get("iban", ""),
        )
    else:
        click.echo("Enter amounts for Modelo 130 (year-to-date cumulative):")
        data = Modelo130Data(
            nif=declarant["nif"],
            apellidos=declarant.get("apellidos", declarant.get("name", "")),
            nombre=declarant.get("nombre", ""),
            exercise=year,
            period=quarter,
            ingresos=Decimal(click.prompt("Ingresos computables (acumulado)", default="0")),
            gastos=Decimal(click.prompt("Gastos deducibles (acumulado)", default="0")),
            pagos_anteriores=Decimal(prev_payments),
            retenciones=Decimal(click.prompt("Retenciones soportadas", default="0")),
            cuenta_iban=config.get("iban", ""),
        )

    boe = generate_130_boe(data)

    if output:
        output.write_text(boe)
        click.echo(f"Written to {output}")
    else:
        click.echo(boe)

    click.echo(f"\n--- Summary ---")
    click.echo(f"Ingresos acum.:     {data.ingresos:>10.2f} €")
    click.echo(f"Gastos acum.:       {data.gastos:>10.2f} €")
    click.echo(f"Rend. neto [03]:    {data.rendimiento_neto:>10.2f} €")
    click.echo(f"20% pago [04]:      {data.pago_20_pct:>10.2f} €")
    click.echo(f"Pagos ant. [05]:    {data.pagos_anteriores:>10.2f} €")
    click.echo(f"Resultado [19]:     {data.resultado:>10.2f} €")
    click.echo(f"Tipo declaración:   {data.tipo_declaracion}")


@main.group()
def submit() -> None:
    """Submit declarations to AEAT."""


@submit.command("303")
@click.option("--year", required=True, type=int)
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.option("--file", "boe_file", type=click.Path(exists=True, path_type=Path), required=True)
@click.option("--dry-run", is_flag=True, help="Validate only, don't submit.")
@click.pass_context
def submit_303(
    ctx: click.Context,
    year: int,
    quarter: str,
    boe_file: Path,
    dry_run: bool,
) -> None:
    """Submit Modelo 303 via Servicios Comunes."""
    config = _load_config(ctx.obj["config_path"])
    cert_cfg = CertificateConfig(
        pfx_path=Path(config["certificate"]["pfx_path"]),
        password=config["certificate"]["password"],
    )

    boe_content = boe_file.read_text()
    testing = config.get("testing", True)

    from .submit import ServiciosComunesClient

    with ServiciosComunesClient(cert_cfg, testing=testing) as client:
        if dry_run:
            click.echo("Validating (dry run)...")
            result = client.validate("303", boe_content)
        else:
            click.echo("Submitting to AEAT...")
            result = client.submit("303", boe_content)

    if result.success:
        click.echo(f"Success! CSV: {result.csv}")
        if result.timestamp:
            click.echo(f"Timestamp: {result.timestamp}")
    else:
        click.echo("Submission failed:", err=True)
        for error in result.errors:
            click.echo(f"  - {error}", err=True)
        sys.exit(1)


@submit.command("130")
@click.option("--year", required=True, type=int)
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.option("--file", "boe_file", type=click.Path(exists=True, path_type=Path), required=True)
@click.option("--dry-run", is_flag=True, help="Validate only, don't submit.")
@click.pass_context
def submit_130(
    ctx: click.Context,
    year: int,
    quarter: str,
    boe_file: Path,
    dry_run: bool,
) -> None:
    """Submit Modelo 130 via Servicios Comunes."""
    config = _load_config(ctx.obj["config_path"])
    cert_cfg = CertificateConfig(
        pfx_path=Path(config["certificate"]["pfx_path"]),
        password=config["certificate"]["password"],
    )

    boe_content = boe_file.read_text()
    testing = config.get("testing", True)

    from .submit import ServiciosComunesClient

    with ServiciosComunesClient(cert_cfg, testing=testing) as client:
        if dry_run:
            click.echo("Validating (dry run)...")
            result = client.validate("130", boe_content)
        else:
            click.echo("Submitting to AEAT...")
            result = client.submit("130", boe_content)

    if result.success:
        click.echo(f"Success! CSV: {result.csv}")
    else:
        click.echo("Submission failed:", err=True)
        for error in result.errors:
            click.echo(f"  - {error}", err=True)
        sys.exit(1)
