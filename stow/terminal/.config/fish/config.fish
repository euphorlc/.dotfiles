# Shared shell configuration; mutable state stays in each workspace home.
if test -d /opt/cargo/bin
    fish_add_path --global --path /opt/cargo/bin
end
if test -d "$HOME/.cargo/bin"
    fish_add_path --global --path "$HOME/.cargo/bin"
end

if test -f ~/.config/acheron/workspace.fish
    source ~/.config/acheron/workspace.fish
end
if not set -q WORKSPACE_THEME
    set -gx WORKSPACE_THEME vulcan
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


if status is-interactive
    set -gx COLORTERM truecolor
    switch "$WORKSPACE_THEME"
        case mars
            set -gx STARSHIP_CONFIG ~/.config/starship_mars.toml
            source ~/.config/fish/theme_mars.fish
        case vulcan
            set -gx STARSHIP_CONFIG ~/.config/starship_vulcan.toml
            source ~/.config/fish/theme_vulcan.fish
    end
    if set -q TMUX_TMPDIR
        mkdir -p -- "$TMUX_TMPDIR"
    end
    if command -q starship
        starship init fish | source
    end
    if not set -q TMUX; and test "$TERM_PROGRAM" != vscode; and command -q tmux
        exec tmux new-session -A -s main
    end
end
