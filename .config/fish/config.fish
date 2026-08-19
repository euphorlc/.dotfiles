# ~/.config/fish/config.fish

if status is-interactive
    # Set locale
    set -x LANG "en_US.UTF-8"
    set -x LANGUAGE "en_US.UTF-8"
    set -x LC_ALL "en_US.UTF-8"

    # Add SSH keys via keychain (suppress errors if ~/.ssh is missing)
    set -l ssh_keys (find ~/.ssh -type f -name "id_*" ! -name "*.pub" 2>/dev/null)
    if test -n "$ssh_keys"
        keychain --eval --agents ssh $ssh_keys | source
    end
end

# ─── Aliases ──────────────────────────────────────────────────────────────────
alias ls='ls -lha --color=auto'
alias dir='dir --color=auto'
alias vdir='vdir --color=auto'
alias grep='grep --color=auto'
alias fgrep='fgrep --color=auto'
alias egrep='egrep --color=auto'
alias l='ls -CF'
alias em='emacs -nw'
alias dd='dd status=progress'
alias _s='sudo'
alias _i='sudo -i'
alias fucking='sudo'
alias please='sudo'

# ─── Workspace-specific theme routing for Starship and Fish ───────────────────
switch "$WORKSPACE_THEME"
    case "mars"
        set -gx STARSHIP_CONFIG ~/.config/starship_mars.toml
        source ~/.config/fish/theme_mars.fish

    case "vulcan"
        set -gx STARSHIP_CONFIG ~/.config/starship_vulcan.toml
        source ~/.config/fish/theme_vulcan.fish

    case "*"
        set -gx STARSHIP_CONFIG ~/.config/starship.toml
end

starship init fish | source

# Ensure manual 'tmux' commands use container socket name
if set -q WORKSPACE_THEME
    alias tmux="command tmux -L $WORKSPACE_THEME"
end

# ─── Auto-attach to tmux (skip inside VSCode or existing tmux session) ────────
if status is-interactive
    and test -z "$TMUX"
    and test "$TERM_PROGRAM" != "vscode"
    and test -n "$CONTAINER_ID"

    if test -n "$WORKSPACE_THEME"
        exec tmux -L "$WORKSPACE_THEME" new-session -A -s main
    else
        exec tmux new-session -A -s main
    end
end
