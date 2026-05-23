import re

with open('src/ui.rs', 'r') as f:
    code = f.read()

# Replace columns constraints
old_constraints = "        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])"
new_constraints = "        .constraints([Constraint::Length(35), Constraint::Min(0)])"
code = code.replace(old_constraints, new_constraints)

# Replace make_block calls
code = re.sub(r'make_block\(([^,]+),\s*([^)]+)\)', r'make_block(\1, \2, app.theme())', code)
code = re.sub(r'make_help_line\(([^,]+),\s*([^)]+)\)', r'make_help_line(\1, \2, app.theme())', code)

# Update helper signatures
code = code.replace("fn make_block<'a, T: Into<ratatui::widgets::block::Title<'a>>>(title: T, focused: bool) -> Block<'a> {",
                    "fn make_block<'a, T: Into<ratatui::widgets::block::Title<'a>>>(title: T, focused: bool, theme: &crate::theme::Theme) -> Block<'a> {")
code = code.replace("fn make_help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {",
                    "fn make_help_line<'a>(key: &'a str, desc: &'a str, theme: &crate::theme::Theme) -> Line<'a> {")

# Replace colors in App functions
# Inside draw* functions, we can just use app.theme().color
def replacer(m):
    color = m.group(1)
    if color in ["BG", "SURFACE", "BORDER", "ACCENT", "FG", "MUTED", "GREEN", "RED", "YELLOW", "CYAN"]:
        return f"app.theme().{color.lower()}"
    return m.group(0)

# But wait, in make_block and make_help_line, it's `theme.color`.
# Let's just do a string replace for all color words with `theme.color` inside helpers.
helpers_split = code.split('// ──────────────────────────── Helpers ─────────────────────────────────')
main_code = helpers_split[0]
helpers_code = helpers_split[1]

for color in ["BG", "SURFACE", "BORDER", "ACCENT", "FG", "MUTED", "GREEN", "RED", "YELLOW", "CYAN"]:
    main_code = re.sub(r'\b' + color + r'\b', f'app.theme().{color.lower()}', main_code)
    helpers_code = re.sub(r'\b' + color + r'\b', f'theme.{color.lower()}', helpers_code)

code = main_code + '// ──────────────────────────── Helpers ─────────────────────────────────' + helpers_code

with open('src/ui.rs', 'w') as f:
    f.write(code)

