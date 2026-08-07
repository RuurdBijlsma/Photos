# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///

import argparse
import re
from pathlib import Path

# Match icon="mdi-foo-bar" or icon='mdi-foo-bar' -> :icon="mdiFooBar"
PATTERN_ATTR_DOUBLE = re.compile(r'\bicon="mdi-([a-z0-9-]+)"')
PATTERN_ATTR_SINGLE = re.compile(r"\bicon='mdi-([a-z0-9-]+)'")

# Match string literal 'mdi-foo-bar' or "mdi-foo-bar" -> mdiFooBar
PATTERN_STRING_SINGLE = re.compile(r"'mdi-([a-z0-9-]+)'")
PATTERN_STRING_DOUBLE = re.compile(r'"mdi-([a-z0-9-]+)"')

# Match existing @mdi/js import: import { mdiFoo, mdiBar } from '@mdi/js'
PATTERN_MDI_IMPORT = re.compile(r"import\s+\{([^}]+)\}\s+from\s+['\"]@mdi/js['\"]")


def kebab_to_camel_mdi(kebab_name: str) -> str:
    """Converts 'first-second' or 'check-bold' to 'mdiFirstSecond' or 'mdiCheckBold'."""
    parts = [p for p in kebab_name.split("-") if p]
    return "mdi" + "".join(p.capitalize() for p in parts)


def update_mdi_imports(content: str, new_icons: set[str], is_vue: bool) -> str:
    if not new_icons:
        return content

    import_match = PATTERN_MDI_IMPORT.search(content)

    if import_match:
        existing_str = import_match.group(1)
        existing_icons = {
            name.strip() for name in existing_str.split(",") if name.strip()
        }
        all_icons = sorted(existing_icons.union(new_icons))
        new_import_stmt = f"import {{ {', '.join(all_icons)} }} from '@mdi/js'"
        return (
            content[: import_match.start()]
            + new_import_stmt
            + content[import_match.end() :]
        )

    # No existing @mdi/js import
    all_icons = sorted(new_icons)
    import_stmt = f"import {{ {', '.join(all_icons)} }} from '@mdi/js'"

    if is_vue:
        # Find <script setup ...> or <script ...>
        script_setup_match = re.search(
            r"(<script[^>]*setup[^>]*>)", content, re.IGNORECASE
        )
        script_match = re.search(r"(<script[^>]*>)", content, re.IGNORECASE)
        target_script = script_setup_match or script_match

        if target_script:
            pos = target_script.end()
            return content[:pos] + f"\n{import_stmt}" + content[pos:]
        else:
            return (
                content + f'\n\n<script setup lang="ts">\n{import_stmt}\n</script>\n'
            )
    else:
        # TS / JS file: prepend import at top
        return f"{import_stmt}\n" + content


def process_file(file_path: Path, dry_run: bool = False) -> tuple[bool, set[str]]:
    try:
        content = file_path.read_text(encoding="utf-8")
    except Exception as e:
        print(f"⚠️ Could not read {file_path}: {e}")
        return False, set()

    original_content = content
    found_icons: set[str] = set()

    # 1. Replace template attributes: icon="mdi-foo-bar" -> :icon="mdiFooBar"
    def repl_attr(match: re.Match) -> str:
        icon_name = kebab_to_camel_mdi(match.group(1))
        found_icons.add(icon_name)
        return f':icon="{icon_name}"'

    content = PATTERN_ATTR_DOUBLE.sub(repl_attr, content)
    content = PATTERN_ATTR_SINGLE.sub(repl_attr, content)

    # 2. Replace string literals: 'mdi-foo-bar' -> mdiFooBar
    def repl_str(match: re.Match) -> str:
        icon_name = kebab_to_camel_mdi(match.group(1))
        found_icons.add(icon_name)
        return icon_name

    content = PATTERN_STRING_SINGLE.sub(repl_str, content)
    content = PATTERN_STRING_DOUBLE.sub(repl_str, content)

    if not found_icons and content == original_content:
        return False, set()

    # 3. Insert or update @mdi/js import
    is_vue = file_path.suffix.lower() == ".vue"
    content = update_mdi_imports(content, found_icons, is_vue)

    changed = content != original_content
    if changed and not dry_run:
        file_path.write_text(content, encoding="utf-8")

    return changed, found_icons


def main():
    parser = argparse.ArgumentParser(
        description="Migrate MDI kebab-case icon strings to @mdi/js camelCase imports."
    )
    parser.add_argument(
        "target",
        nargs="?",
        default="web/src",
        help="Target directory or file (default: web/src)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview modifications without writing to disk",
    )
    args = parser.parse_args()

    target_path = Path(args.target)
    if not target_path.exists():
        print(f"❌ Target path '{target_path}' does not exist.")
        return

    extensions = {".vue", ".ts", ".js", ".jsx", ".tsx"}
    files = (
        [target_path]
        if target_path.is_file()
        else [p for p in target_path.rglob("*") if p.suffix.lower() in extensions]
    )

    modified_count = 0
    total_icons_found = 0

    mode_label = "[DRY RUN] " if args.dry_run else ""
    print(f"🚀 {mode_label}Scanning {len(files)} files in '{target_path}'...")

    for file_path in files:
        changed, icons = process_file(file_path, dry_run=args.dry_run)
        if changed:
            modified_count += 1
            total_icons_found += len(icons)
            icons_str = ", ".join(sorted(icons))
            print(f"  ✨ {file_path} -> ({len(icons)} icons: {icons_str})")

    print("\n✅ Migration complete!")
    print(f"   Files modified: {modified_count}")
    print(f"   Icons updated: {total_icons_found}")


if __name__ == "__main__":
    main()