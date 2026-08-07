# /// script
# dependencies = [
#     "rich",
# ]
# ///

import os
import re
import sys
from pathlib import Path
from rich.console import Console

console = Console()

def kebab_to_camel(kebab_str: str) -> str:
    """Converts 'mdi-image-multiple-outline' or 'image-multiple-outline' to 'mdiImageMultipleOutline'."""
    if not kebab_str.startswith("mdi-"):
        kebab_str = "mdi-" + kebab_str
    parts = kebab_str.split("-")
    return parts[0] + "".join(part.capitalize() for part in parts[1:])

def process_file_content(content: str, is_vue: bool) -> tuple[str, set[str], int]:
    new_imports = set()
    replacements_count = 0

    # 1. Attribute string literals: e.g. icon="mdi-refresh", prepend-icon="mdi-calendar-filter", prepend-inner-icon="mdi-text"
    # Matches any attribute ending in 'icon' or '-icon' equal to "mdi-..."
    attr_pattern = re.compile(r'\b([a-zA-Z0-9-]*icon)="mdi-([a-z0-9-]+)"')
    def attr_replacer(match):
        nonlocal replacements_count
        attr_name = match.group(1)
        kebab_icon = match.group(2)
        var_name = kebab_to_camel(kebab_icon)
        new_imports.add(var_name)
        replacements_count += 1
        return f':{attr_name}="{var_name}"'

    content = attr_pattern.sub(attr_replacer, content)

    # 2. Tag content in <v-icon>: e.g. <v-icon size="20"> mdi-cloud-outline </v-icon>
    v_icon_pattern = re.compile(r'<v-icon([^>]*)>\s*mdi-([a-z0-9-]+)\s*</v-icon>', re.MULTILINE)
    def v_icon_replacer(match):
        nonlocal replacements_count
        attrs = match.group(1)
        kebab_icon = match.group(2)
        var_name = kebab_to_camel(kebab_icon)
        new_imports.add(var_name)
        replacements_count += 1
        return f'<v-icon{attrs} :icon="{var_name}"></v-icon>'

    content = v_icon_pattern.sub(v_icon_replacer, content)

    # 3. Object property string values: e.g. icon: 'mdi-image-multiple-outline' or prependIcon: "mdi-plus"
    prop_pattern = re.compile(r'\b([a-zA-Z0-9_]*[iI]con):\s*[\'"]mdi-([a-z0-9-]+)[\'"]')
    def prop_replacer(match):
        nonlocal replacements_count
        prop_name = match.group(1)
        kebab_icon = match.group(2)
        var_name = kebab_to_camel(kebab_icon)
        new_imports.add(var_name)
        replacements_count += 1
        return f'{prop_name}: {var_name}'

    content = prop_pattern.sub(prop_replacer, content)

    # If no replacements were made, return unchanged
    if not new_imports:
        return content, new_imports, 0

    # 4. Inject or update @mdi/js imports
    content = update_mdi_imports(content, is_vue, new_imports)

    return content, new_imports, replacements_count

def update_mdi_imports(content: str, is_vue: bool, new_imports: set[str]) -> str:
    import_regex = re.compile(r'import\s+\{\s*([^}]+)\s*\}\s+from\s+[\'"]@mdi\/js[\'"]')
    match = import_regex.search(content)

    existing_imports = set()
    if match:
        # Extract existing imported names
        raw_names = match.group(1).split(",")
        existing_imports = {n.strip() for n in raw_names if n.strip()}

    all_imports = sorted(existing_imports | new_imports)
    import_statement = f"import {{ {', '.join(all_imports)} }} from '@mdi/js'"

    if match:
        # Replace existing import line
        return content[:match.start()] + import_statement + content[match.end():]

    # Insert new import statement
    if is_vue:
        # Look for <script setup ...> or <script ...>
        script_match = re.search(r'(<script[^>]*>)', content)
        if script_match:
            insert_pos = script_match.end()
            return content[:insert_pos] + f"\n{import_statement}" + content[insert_pos:]
        else:
            # Vue file with no script tag -> prepend script block at top
            return f"<script setup lang=\"ts\">\n{import_statement}\n</script>\n\n" + content
    else:
        # Regular .ts or .js file -> insert at top
        return f"{import_statement}\n" + content

def main():
    target_dir = Path("src")
    if len(sys.argv) > 1:
        target_dir = Path(sys.argv[1])

    if not target_dir.exists():
        console.print(f"[bold red]Error:[/bold red] Directory '{target_dir}' does not exist.")
        sys.exit(1)

    console.print(f"[bold blue]Scanning for MDI icons in:[/bold blue] {target_dir.resolve()}\n")

    file_extensions = {".vue", ".ts", ".js"}
    modified_files = 0
    total_replacements = 0

    for root, _, files in os.walk(target_dir):
        for file in files:
            file_path = Path(root) / file
            if file_path.suffix not in file_extensions:
                continue

            try:
                original_content = file_path.read_text(encoding="utf-8")
                is_vue = file_path.suffix == ".vue"

                new_content, imports, count = process_file_content(original_content, is_vue)

                if count > 0 and new_content != original_content:
                    file_path.write_text(new_content, encoding="utf-8")
                    modified_files += 1
                    total_replacements += count
                    console.print(
                        f"[green]✔ Modified[/green] [bold]{file_path.relative_to(target_dir)}[/bold] "
                        f"({count} icon(s) updated: [dim]{', '.join(sorted(imports))}[/dim])"
                    )

            except Exception as e:
                console.print(f"[red]Failed to process {file_path}: {e}[/red]")

    console.print(f"\n[bold green]Done![/bold green] Updated {total_replacements} icon(s) across {modified_files} file(s).")

if __name__ == "__main__":
    main()