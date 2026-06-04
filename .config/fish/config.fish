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

# ─── OS-specific theme, Starship, and Alacritty profile ───────────────────────
if test -f /etc/os-release
    # Kali Linux - Ares (PenTest)
    if grep -qi "kali" /etc/os-release
        source ~/.config/fish/theme_ares.fish
        set -gx STARSHIP_CONFIG ~/.config/starship_ares.toml

    # Ubuntu - Hephaestus (Dev)
    else if grep -qi "ubuntu" /etc/os-release
        source ~/.config/fish/theme_hephaestus.fish
        set -gx STARSHIP_CONFIG ~/.config/starship_hephaestus.toml
    end
end

starship init fish | source

# ─── Auto-attach to tmux (skip inside VSCode or existing tmux session) ────────
if status is-interactive
    and test -z "$TMUX"
    and test "$TERM_PROGRAM" != "vscode"
    exec tmux new-session -A -s main
end
