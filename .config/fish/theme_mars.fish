# ~/.config/fish/theme_mars.fish

# Primary command styling
set -g fish_color_normal normal
set -g fish_color_command yellow --bold --underline
set -g fish_color_param normal
set -g fish_color_quote normal
set -g fish_color_keyword yellow --bold

# Syntax and errors
set -g fish_color_error red --bold
set -g fish_color_escape red
set -g fish_color_operator yellow
set -g fish_color_end green
set -g fish_color_redirection green
set -g fish_color_comment black --bold
set -g fish_color_valid_path --underline

# Autosuggestions and selection
set -g fish_color_autosuggestion black --bold
set -g fish_color_selection --background=blue
set -g fish_color_search_match --background=magenta
set -g fish_color_history_current --bold
set -g fish_color_cancel -r

# Pager (tab-completion menu) colors
set -g fish_pager_color_background normal
set -g fish_pager_color_prefix yellow --bold --underline
set -g fish_pager_color_completion normal
set -g fish_pager_color_description black --bold
set -g fish_pager_color_progress brwhite --background=blue
set -g fish_pager_color_selected_background --background=yellow
set -g fish_pager_color_selected_prefix black --bold
set -g fish_pager_color_selected_completion black --bold
set -g fish_pager_color_selected_description black
