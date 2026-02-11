# oag — OpenAPI 3.x code generator demo
Output doc/demo.gif

Set Shell "zsh"
Set FontFamily "MonaspiceNe Nerd Font"
Set FontSize 16
Set Width 1200
Set Height 700
Set Padding 24
Set Theme { "name": "Cyberdream", "black": "#16181a", "red": "#ff6e5e", "green": "#5eff6c", "yellow": "#f1ff5e", "blue": "#5ea1ff", "magenta": "#bd5eff", "cyan": "#5ef1ff", "white": "#ffffff", "brightBlack": "#3c4048", "brightRed": "#ff6e5e", "brightGreen": "#5eff6c", "brightYellow": "#f1ff5e", "brightBlue": "#5ea1ff", "brightMagenta": "#bd5eff", "brightCyan": "#5ef1ff", "brightWhite": "#ffffff", "background": "#16181a", "foreground": "#ffffff", "selection": "#3c4048", "cursor": "#ffffff" }
Set WindowBar "Rings"
Set TypingSpeed 50ms
Set CursorBlink false

# ── Setup (hidden from recording) ──────────────────
Hide
Type "export PATH=./target/release:$PATH"
Enter
Sleep 500ms
Type "export SPEC=crates/oag-core/tests/fixtures/petstore-3.2.yaml"
Enter
Sleep 500ms
Type "export SSE_SPEC=crates/oag-core/tests/fixtures/sse-chat.yaml"
Enter
Sleep 500ms
Type "export PS1='❯ '"
Enter
Sleep 500ms
Type "splash() { clear; print ''; print '  \e[90m╭──────────────────────────────────────────────╮\e[0m'; print '  \e[90m│\e[0m                                              \e[90m│\e[0m'; print '  \e[90m│\e[0m        \e[35m✦\e[0m  \e[1;36mOpenAPI Generator\e[0m \e[36m(oag)\e[0m  \e[35m✦\e[0m         \e[90m│\e[0m'; print '  \e[90m│\e[0m                                              \e[90m│\e[0m'; print '  \e[90m│\e[0m         \e[34mgithub.com/urmzd\e[0m \e[90m·\e[0m \e[34murmzd.com\e[0m         \e[90m│\e[0m'; print '  \e[90m│\e[0m                                              \e[90m│\e[0m'; print '  \e[90m╰──────────────────────────────────────────────╯\e[0m'; print ''; }"
Enter
Sleep 500ms
Type "splash"
Enter
Sleep 500ms
Show

# ── Splash pause (let viewer read) ────────────────
Sleep 3s

# ── CLI overview ───────────────────────────────────
Type "oag --help"
Enter
Sleep 4s

# ── Validate a spec ────────────────────────────────
Type "oag validate -i $SPEC"
Enter
Sleep 3s

# ── Generate TypeScript client ─────────────────────
Type "oag generate -i $SPEC -o /tmp/petstore -t typescript"
Enter
Sleep 3s

# ── Show generated files ──────────────────────────
Type "ls /tmp/petstore/"
Enter
Sleep 2s

# ── Clear and preview generated types ──────────────
Ctrl+L
Sleep 500ms
Type "head -n 25 /tmp/petstore/types.ts"
Enter
Sleep 4s

# ── Preview generated client ──────────────────────
Ctrl+L
Sleep 500ms
Type "head -n 40 /tmp/petstore/client.ts"
Enter
Sleep 5s

# ── Generate React hooks from SSE chat spec ───────
Ctrl+L
Sleep 500ms
Type "oag generate -i $SSE_SPEC -o /tmp/sse-chat -t react"
Enter
Sleep 3s

# ── Show generated React files ────────────────────
Type "ls /tmp/sse-chat/"
Enter
Sleep 2s

# ── Preview generated hooks ──────────────────────
Ctrl+L
Sleep 500ms
Type "head -n 40 /tmp/sse-chat/hooks.ts"
Enter
Sleep 5s
