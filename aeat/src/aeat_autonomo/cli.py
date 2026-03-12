"""CLI entrypoint for AEAT autónomo tax automation.

Usage:
    aeat generate 303 --quarter 1T --year 2026
    aeat generate 130 --quarter 1T --year 2026
    aeat submit 303 --quarter 1T --year 2026 [--dry-run]
    aeat quipu-totals --quarter 1T --year 2026
    aeat simulate --year 2024
"""

from __future__ import annotations

import json
import sys
from datetime import date
from decimal import Decimal
from pathlib import Path
from typing import TYPE_CHECKING

import click

from .config import AeatConfig
from .logging import setup_logging
from .clients.quipu import QuipuClient
from .models import Modelo130Data, Modelo303Data, encode_boe, generate_130_boe, generate_303_boe

if TYPE_CHECKING:
    from .clients.aeat import PresentacionDirectaClient, SubmissionResult


def _load_config(config_path: Path) -> AeatConfig:
    """Load configuration from a JSON file."""
    if not config_path.exists():
        click.echo(f"Config file not found: {config_path}", err=True)
        click.echo("Create one with: aeat init", err=True)
        sys.exit(1)
    return AeatConfig.from_file(config_path)


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
    setup_logging(json_output=False)
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

    q_num = int(quarter[0])
    with QuipuClient(config.quipu) as client:
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

    if from_quipu:
        q_num = int(quarter[0])
        with QuipuClient(config.quipu) as client:
            totals = client.get_quarterly_totals(year, q_num)

        data = Modelo303Data(
            nif=config.declarant.nif,
            nombre_razon=config.declarant.nombre_completo,
            exercise=year,
            period=quarter,
            base_21=totals.total_income_gross,
            cuota_21=totals.total_vat_collected,
            base_deducible_interior=totals.total_expenses_gross,
            cuota_deducible_interior=totals.total_vat_deductible,
            cuenta_iban=config.iban,
        )
    else:
        click.echo("Enter amounts for Modelo 303:")
        data = Modelo303Data(
            nif=config.declarant.nif,
            nombre_razon=config.declarant.nombre_completo,
            exercise=year,
            period=quarter,
            base_21=Decimal(click.prompt("Base imponible 21%", default="0")),
            cuota_21=Decimal(click.prompt("Cuota IVA 21%", default="0")),
            base_10=Decimal(click.prompt("Base imponible 10%", default="0")),
            cuota_10=Decimal(click.prompt("Cuota IVA 10%", default="0")),
            base_deducible_interior=Decimal(
                click.prompt("Base IVA deducible (gastos)", default="0")
            ),
            cuota_deducible_interior=Decimal(
                click.prompt("Cuota IVA deducible", default="0")
            ),
            cuenta_iban=config.iban,
        )

    boe = generate_303_boe(data)

    if output:
        output.write_bytes(encode_boe(boe))
        click.echo(f"Written to {output}")
    else:
        click.echo(boe)

    click.echo(f"\n--- Summary ---")
    click.echo(f"IVA devengado [27]: {data.total_cuota_devengada:>10.2f} €")
    click.echo(f"IVA deducible [45]: {data.total_a_deducir:>10.2f} €")
    click.echo(f"Resultado     [69]: {data.resultado:>10.2f} €")
    click.echo(f"Liquidación   [71]: {data.resultado_liquidacion:>10.2f} €")
    click.echo(f"Tipo declaración:   {data.tipo_declaracion}")


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

    if from_quipu:
        # For 130, we need ACCUMULATED income from Q1 through current quarter
        q_num = int(quarter[0])
        total_income = Decimal("0")
        total_expenses = Decimal("0")

        with QuipuClient(config.quipu) as client:
            for q in range(1, q_num + 1):
                totals = client.get_quarterly_totals(year, q)
                total_income += totals.total_income_gross
                total_expenses += totals.total_expenses_gross

        data = Modelo130Data(
            nif=config.declarant.nif,
            apellidos=config.declarant.apellidos,
            nombre=config.declarant.nombre,
            exercise=year,
            period=quarter,
            ingresos=total_income,
            gastos=total_expenses,
            pagos_anteriores=Decimal(prev_payments),
            cuenta_iban=config.iban,
        )
    else:
        click.echo("Enter amounts for Modelo 130 (year-to-date cumulative):")
        data = Modelo130Data(
            nif=config.declarant.nif,
            apellidos=config.declarant.apellidos,
            nombre=config.declarant.nombre,
            exercise=year,
            period=quarter,
            ingresos=Decimal(click.prompt("Ingresos computables (acumulado)", default="0")),
            gastos=Decimal(click.prompt("Gastos deducibles (acumulado)", default="0")),
            pagos_anteriores=Decimal(prev_payments),
            retenciones=Decimal(click.prompt("Retenciones soportadas", default="0")),
            cuenta_iban=config.iban,
        )

    boe = generate_130_boe(data)

    if output:
        output.write_bytes(encode_boe(boe))
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


@main.command("simulate")
@click.option("--year", type=int, default=None, help="Fiscal year to simulate (default: current year).")
@click.option(
    "--quarters",
    default="1T,2T,3T,4T",
    help="Quarters to simulate (comma-separated). Default: all.",
)
@click.option("--output-dir", "-o", type=click.Path(path_type=Path), default=None,
              help="Directory to save generated BOE files.")
@click.pass_context
def simulate(ctx: click.Context, year: int | None, quarters: str, output_dir: Path | None) -> None:
    """Simulate a full fiscal year using Quipu data.

    Fetches historical data from Quipu for each quarter and generates
    both Modelo 303 (VAT) and Modelo 130 (IRPF) declarations, showing
    what each filing would have looked like.

    No certificate needed — nothing is submitted to AEAT.

    Examples:

        aeat simulate --year 2024

        aeat simulate --year 2025 --quarters 1T,2T

        aeat simulate --year 2024 -o ./sim-2024/
    """
    if year is None:
        year = date.today().year

    config = _load_config(ctx.obj["config_path"])
    iban = config.iban

    quarter_list = [q.strip() for q in quarters.split(",")]
    for q in quarter_list:
        if q not in ("1T", "2T", "3T", "4T"):
            click.echo(f"Invalid quarter: {q}", err=True)
            sys.exit(1)

    if output_dir:
        output_dir.mkdir(parents=True, exist_ok=True)

    # Determine which quarters we need to fetch (for 130 accumulation,
    # we always need Q1 through the latest requested quarter)
    max_q = max(int(q[0]) for q in quarter_list)
    quarters_to_fetch = [f"{i}T" for i in range(1, max_q + 1)]

    # Fetch quarterly data from Quipu
    click.echo(f"Fetching Quipu data for {year}...\n")
    quarterly_data = {}
    with QuipuClient(config.quipu) as client:
        for q in quarters_to_fetch:
            q_num = int(q[0])
            quarterly_data[q] = client.get_quarterly_totals(year, q_num)

    # Track cumulative values for Modelo 130
    accum_income = Decimal("0")
    accum_expenses = Decimal("0")
    accum_130_payments = Decimal("0")

    # Annual totals (only for displayed quarters)
    annual_vat_collected = Decimal("0")
    annual_vat_deductible = Decimal("0")
    annual_income = Decimal("0")
    annual_expenses = Decimal("0")

    for q in quarters_to_fetch:
        totals = quarterly_data[q]
        accum_income += totals.total_income_gross
        accum_expenses += totals.total_expenses_gross

        # Build Modelo 130 for this quarter (needed for payment accumulation)
        m130 = Modelo130Data(
            nif=config.declarant.nif,
            apellidos=config.declarant.apellidos,
            nombre=config.declarant.nombre,
            exercise=year,
            period=q,
            ingresos=accum_income,
            gastos=accum_expenses,
            pagos_anteriores=accum_130_payments,
            cuenta_iban=iban,
        )

        if q not in quarter_list:
            # Not a displayed quarter — just accumulate 130 payments and skip
            if m130.resultado > 0:
                accum_130_payments += m130.resultado
            continue

        # Track annual totals for displayed quarters
        annual_vat_collected += totals.total_vat_collected
        annual_vat_deductible += totals.total_vat_deductible
        annual_income += totals.total_income_gross
        annual_expenses += totals.total_expenses_gross

        click.echo(f"{'=' * 60}")
        click.echo(f"  {year} {q}")
        click.echo(f"{'=' * 60}")

        # Quipu raw data
        click.echo(f"\n  Quipu data:")
        click.echo(f"    Income (base):      {totals.total_income_gross:>12.2f} €")
        click.echo(f"    VAT collected:      {totals.total_vat_collected:>12.2f} €")
        click.echo(f"    Expenses (base):    {totals.total_expenses_gross:>12.2f} €")
        click.echo(f"    VAT deductible:     {totals.total_vat_deductible:>12.2f} €")

        # --- Modelo 303 ---
        m303 = Modelo303Data(
            nif=config.declarant.nif,
            nombre_razon=config.declarant.nombre_completo,
            exercise=year,
            period=q,
            base_21=totals.total_income_gross,
            cuota_21=totals.total_vat_collected,
            base_deducible_interior=totals.total_expenses_gross,
            cuota_deducible_interior=totals.total_vat_deductible,
            cuenta_iban=iban,
        )

        click.echo(f"\n  Modelo 303 (IVA):")
        click.echo(f"    IVA devengado [27]:   {m303.total_cuota_devengada:>12.2f} €")
        click.echo(f"    IVA deducible [45]:   {m303.total_a_deducir:>12.2f} €")
        click.echo(f"    Resultado     [69]:   {m303.resultado:>12.2f} €")
        click.echo(f"    Liquidación   [71]:   {m303.resultado_liquidacion:>12.2f} €")
        click.echo(f"    Tipo:                 {m303.tipo_declaracion}")

        # --- Modelo 130 ---
        click.echo(f"\n  Modelo 130 (IRPF):")
        click.echo(f"    Ingresos acum. [01]:  {m130.ingresos:>12.2f} €")
        click.echo(f"    Gastos acum.   [02]:  {m130.gastos:>12.2f} €")
        click.echo(f"    Rend. neto     [03]:  {m130.rendimiento_neto:>12.2f} €")
        click.echo(f"    20% pago       [04]:  {m130.pago_20_pct:>12.2f} €")
        click.echo(f"    Pagos ant.     [05]:  {m130.pagos_anteriores:>12.2f} €")
        click.echo(f"    Resultado      [19]:  {m130.resultado:>12.2f} €")
        click.echo(f"    Tipo:                 {m130.tipo_declaracion}")

        # Update accumulated 130 payments
        if m130.resultado > 0:
            accum_130_payments += m130.resultado

        # Save BOE files if requested
        if output_dir:
            boe_303 = generate_303_boe(m303)
            boe_130 = generate_130_boe(m130)
            path_303 = output_dir / f"modelo303_{year}_{q}.boe"
            path_130 = output_dir / f"modelo130_{year}_{q}.boe"
            path_303.write_bytes(encode_boe(boe_303))
            path_130.write_bytes(encode_boe(boe_130))
            click.echo(f"\n    Saved: {path_303}")
            click.echo(f"    Saved: {path_130}")

        click.echo()

    # Annual summary
    click.echo(f"{'=' * 60}")
    click.echo(f"  {year} ANNUAL SUMMARY")
    click.echo(f"{'=' * 60}")
    click.echo(f"    Total income:         {annual_income:>12.2f} €")
    click.echo(f"    Total expenses:       {annual_expenses:>12.2f} €")
    click.echo(f"    Net income (IRPF):    {annual_income - annual_expenses:>12.2f} €")
    click.echo(f"    Total VAT collected:  {annual_vat_collected:>12.2f} €")
    click.echo(f"    Total VAT deducted:   {annual_vat_deductible:>12.2f} €")
    click.echo(f"    Net VAT paid:         {annual_vat_collected - annual_vat_deductible:>12.2f} €")
    click.echo(f"    IRPF advance paid:    {accum_130_payments:>12.2f} €")


@main.command("serve")
@click.option("--host", default="0.0.0.0", envvar="AEAT_HOST", help="Bind host.")
@click.option("--port", default=8000, type=int, envvar="AEAT_PORT", help="Bind port.")
@click.option("--reload", "use_reload", is_flag=True, help="Enable auto-reload (dev mode).")
@click.pass_context
def serve(ctx: click.Context, host: str, port: int, use_reload: bool) -> None:
    """Start the REST API server."""
    try:
        import uvicorn
    except ImportError:
        click.echo("API dependencies not installed. Run: pip install aeat-autonomo[api]", err=True)
        sys.exit(1)

    # Pass config path via env so the API can pick it up
    import os
    os.environ.setdefault("AEAT_CONFIG", str(ctx.obj["config_path"]))

    uvicorn.run(
        "aeat_autonomo.api:app",
        host=host,
        port=port,
        reload=use_reload,
    )


@main.group()
def submit() -> None:
    """Submit declarations to AEAT."""


def _make_submit_client(config: AeatConfig) -> PresentacionDirectaClient:
    """Create a PresentacionDirectaClient from config."""
    from .clients.aeat import PresentacionDirectaClient

    return PresentacionDirectaClient(
        config.certificate,
        nif_presentador=config.declarant.nif,
        nombre_presentador=config.declarant.nombre_completo,
        testing=config.testing,
    )


def _print_submit_result(result: SubmissionResult, dry_run: bool = False) -> None:
    """Print submission/validation result."""
    if result.success:
        if dry_run:
            click.echo("Validation passed!")
            if result.pdf_base64:
                click.echo("PDF preview available (use --save-pdf to save).")
        else:
            click.echo(f"Submitted successfully!")
            click.echo(f"  CSV:          {result.csv}")
            click.echo(f"  Justificante: {result.justificante}")
            click.echo(f"  Fecha/Hora:   {result.fecha} {result.hora}")
            if result.pdf_url:
                click.echo(f"  PDF:          {result.pdf_url}")
        if result.warnings:
            click.echo("Warnings:")
            for w in result.warnings:
                click.echo(f"  - {w}")
    else:
        click.echo("Failed:", err=True)
        for error in result.errors:
            click.echo(f"  - {error}", err=True)
        sys.exit(1)


@submit.command("303")
@click.option("--year", required=True, type=int)
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.option("--file", "boe_file", type=click.Path(exists=True, path_type=Path), required=True)
@click.option("--nrc", default="", help="NRC payment reference (required for tipo=Ingreso).")
@click.option("--dry-run", is_flag=True, help="Validate only, don't submit.")
@click.option("--save-pdf", type=click.Path(path_type=Path), default=None)
@click.pass_context
def submit_303(
    ctx: click.Context,
    year: int,
    quarter: str,
    boe_file: Path,
    nrc: str,
    dry_run: bool,
    save_pdf: Path | None,
) -> None:
    """Submit Modelo 303 via Presentación Directa (JSON API)."""
    config = _load_config(ctx.obj["config_path"])
    boe_content = boe_file.read_text(encoding="iso-8859-1")

    with _make_submit_client(config) as client:
        if dry_run:
            click.echo("Validating against AEAT test environment...")
            result = client.validate("303", str(year), quarter, boe_content)
            if result.success and save_pdf:
                client.save_validation_pdf(result, save_pdf)
                click.echo(f"PDF saved to {save_pdf}")
        else:
            click.echo("Submitting to AEAT...")
            result = client.submit("303", str(year), quarter, boe_content, nrc=nrc)

    _print_submit_result(result, dry_run=dry_run)


@submit.command("130")
@click.option("--year", required=True, type=int)
@click.option("--quarter", required=True, type=click.Choice(["1T", "2T", "3T", "4T"]))
@click.option("--file", "boe_file", type=click.Path(exists=True, path_type=Path), required=True)
@click.option("--nrc", default="", help="NRC payment reference (required for tipo=Ingreso).")
@click.option("--dry-run", is_flag=True, help="Validate only, don't submit.")
@click.option("--save-pdf", type=click.Path(path_type=Path), default=None)
@click.pass_context
def submit_130(
    ctx: click.Context,
    year: int,
    quarter: str,
    boe_file: Path,
    nrc: str,
    dry_run: bool,
    save_pdf: Path | None,
) -> None:
    """Submit Modelo 130 via Presentación Directa (JSON API)."""
    config = _load_config(ctx.obj["config_path"])
    boe_content = boe_file.read_text(encoding="iso-8859-1")

    with _make_submit_client(config) as client:
        if dry_run:
            click.echo("Validating against AEAT test environment...")
            result = client.validate("130", str(year), quarter, boe_content)
            if result.success and save_pdf:
                client.save_validation_pdf(result, save_pdf)
                click.echo(f"PDF saved to {save_pdf}")
        else:
            click.echo("Submitting to AEAT...")
            result = client.submit("130", str(year), quarter, boe_content, nrc=nrc)

    _print_submit_result(result, dry_run=dry_run)
