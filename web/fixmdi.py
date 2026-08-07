# /// script
# dependencies = [
#     "rich",
# ]
# ///

import re
import sys
from pathlib import Path
from rich.console import Console

console = Console()

# Pattern matching `@mdi/js` import blocks (single or multi-line)
MDI_IMPORT_PATTERN = re.compile(
    r"import\s*\{([\s\S]*?)\}\s*from\s*['\"]@mdi/js['\"]",
    re.MULTILINE,
)


def pascal_to_kebab(name: str) -> str:
    """Convert a PascalCase or camelCase icon name (without 'mdi') to kebab-case."""
    # Insert hyphen before capital letters
    s1 = re.sub(r"(.)([A-Z][a-z]+)", r"\1-\2", name)
    s2 = re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", s1)
    return s2.lower()


def transform_file_content(content: str) -> tuple[str, int]:
    """Transforms @mdi/js imports and usages to unplugin-icons format."""
    matches = list(MDI_IMPORT_PATTERN.finditer(content))
    if not matches:
        return content, 0

    icons_found: list[tuple[str, str, str]] = []  # (old_name, new_component_name, kebab_path)

    # Process each import block
    new_content = content
    for match in reversed(matches):
        block_text = match.group(1)
        # Extract all icon identifier names (e.g., mdiAlertCircleOutline)
        extracted_names = re.findall(r"\b(mdi[A-Za-z0-9_]+)\b", block_text)

        new_import_lines = []
        for old_name in extracted_names:
            # Strip 'mdi' prefix to get icon base name
            base_name = old_name[3:]
            # PascalCase component name for script/template usage (e.g., MdiAlertCircleOutline)
            new_component_name = f"Mdi{base_name}"
            # Kebab-case icon path for unplugin-icons (e.g., alert-circle-outline)
            kebab_path = pascal_to_kebab(base_name)

            icons_found.append((old_name, new_component_name, kebab_path))
            new_import_lines.append(
                f"import {new_component_name} from '~icons/mdi/{kebab_path}'"
            )

        replacement_imports = "\n".join(new_import_lines)
        start, end = match.span()
        new_content = new_content[:start] + replacement_imports + new_content[end:]

    # Replace all usage occurrences of old names (e.g. mdiAlertCircleOutline -> MdiAlertCircleOutline)
    # Using word boundary (\b) to avoid partial variable name replacements
    for old_name, new_component_name, _ in icons_found:
        new_content = re.sub(
            r"\b" + re.escape(old_name) + r"\b",
            new_component_name,
            new_content,
        )

    return new_content, len(icons_found)


def main():
    target_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("src")

    if not target_dir.exists():
        console.print(f"[bold red]Target directory '{target_dir}' not found![/bold red]")
        sys.exit(1)

    extensions = {".vue", ".ts", ".tsx", ".js", ".jsx"}
    files_to_process = [
        f for f in target_dir.rglob("*") if f.is_file() and f.suffix in extensions
    ]

    console.print(
        f"[bold blue]Scanning {len(files_to_process)} files in '{target_dir}'...[/bold blue]\n"
    )

    modified_files_count = 0
    total_icons_migrated = 0

    for file_path in files_to_process:
        try:
            original_content = file_path.read_text(encoding="utf-8")
            transformed, icon_count = transform_file_content(original_content)

            if icon_count > 0 and transformed != original_content:
                file_path.write_text(transformed, encoding="utf-8")
                modified_files_count += 1
                total_icons_migrated += icon_count
                console.print(
                    f"[green]✔ Updated:[/green] {file_path.relative_to(target_dir.parent)} "
                    f"([dim]{icon_count} icon(s)[/dim])"
                )
        except Exception as e:
            console.print(f"[bold red]Error processing {file_path}:[/bold red] {e}")

    console.print("\n[bold green]Migration complete![/bold green]")
    console.print(f"• [bold]Files updated:[/bold] {modified_files_count}")
    console.print(f"• [bold]Icons transformed:[/bold] {total_icons_migrated}\n")


if __name__ == "__main__":
    main()