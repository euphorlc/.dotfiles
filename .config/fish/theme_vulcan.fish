# ~/.config/fish/theme_vulcan.fish

# Primary syntax highlighting
set -g fish_color_normal DAF2E9          # Pale Ice Mint
set -g fish_color_command 95E0CC         # Bright Seafoam Green
set -g fish_color_keyword 95E0CC         # Bright Seafoam Green
set -g fish_color_quote 39707A           # Muted Teal
set -g fish_color_redirection F14E52     # Bright Coral Red
set -g fish_color_end 23495D             # Deep Slate Cyan
set -g fish_color_error F14E52 --bold    # Bright Coral Red
set -g fish_color_param DAF2E9           # Pale Ice Mint
set -g fish_color_comment 39707A         # Muted Teal
set -g fish_color_operator F14E52        # Bright Coral Red
set -g fish_color_escape F14E52          # Bright Coral Red
set -g fish_color_valid_path --underline

# Autosuggestions and selection
set -g fish_color_autosuggestion 39707A
set -g fish_color_selection DAF2E9 --bold --background=23495D
set -g fish_color_search_match 1C2638 --background=95E0CC
set -g fish_color_history_current --bold
set -g fish_color_cancel -r

# Pager (tab-completion menu) colors
set -g fish_pager_color_background normal
set -g fish_pager_color_prefix 95E0CC --bold --underline
set -g fish_pager_color_completion DAF2E9
set -g fish_pager_color_description 39707A
set -g fish_pager_color_progress DAF2E9 --background=23495D

# Crimson highlight for selected items
set -g fish_pager_color_selected_background --background=9B222B
set -g fish_pager_color_selected_prefix DAF2E9 --bold
set -g fish_pager_color_selected_completion DAF2E9 --bold
set -g fish_pager_color_selected_description 95E0CC
