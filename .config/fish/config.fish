# ~/.config/fish/config.fish

if status is-interactive
  # Commands to run in interactive sessions can go here
  # Set locale
  set -x LANG "en_US.UTF-8"
  set -x LANGUAGE "en_US.UTF-8"
  set -x LC_ALL "en_US.UTF-8"

  # Add SSH keys via keychain
  set -l ssh_keys (find ~/.ssh -type f -name "id_*" ! -name "*.pub")
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

# ─── Workspace-specific theme routing for Starship and Fish ───────────────────────
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

# ─── Auto-attach to tmux (skip inside VSCode or existing tmux session) ────────
if status is-interactive
    and test -z "$TMUX"
    and test "$TERM_PROGRAM" != "vscode"
    if test -n "$WORKSPACE_THEME"
        set -gx TMUX_TMPDIR "/tmp/tmux-$WORKSPACE_THEME"
    end
    exec tmux new-session -A -s main
end
