# ~/.config/fish/theme_vulcan.fish

# Primary syntax highlighting
set -g fish_color_normal normal
set -g fish_color_command green
set -g fish_color_keyword green
set -g fish_color_quote magenta      # Neon Purple
set -g fish_color_redirection red
set -g fish_color_end yellow         # Orange
set -g fish_color_error brwhite
set -g fish_color_param blue         # Light Purple
set -g fish_color_comment brblack
set -g fish_color_operator red
set -g fish_color_escape red
set -g fish_color_valid_path --underline

# Autosuggestions and selection
set -g fish_color_autosuggestion brblack
set -g fish_color_selection white --bold --background=brblack
set -g fish_color_search_match yellow --background=brblack
set -g fish_color_history_current --bold
set -g fish_color_cancel -r

# Pager (tab-completion menu) colors
set -g fish_pager_color_background normal
set -g fish_pager_color_prefix green --bold --underline
set -g fish_pager_color_completion normal
set -g fish_pager_color_description blue
set -g fish_pager_color_progress brwhite --background=magenta

# Custom red highlight for selected items
set -g fish_pager_color_selected_background --background=red
set -g fish_pager_color_selected_prefix white --bold
set -g fish_pager_color_selected_completion white --bold
set -g fish_pager_color_selected_description brwhite
