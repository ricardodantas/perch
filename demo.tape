# VHS tape for Perch screenshots
Output assets/perch-demo.gif

Set FontFamily "JetBrains Mono"
Set FontSize 14
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"

Type "./target/release/perch --demo"
Enter
Sleep 1500ms

# Take main screenshot
Screenshot assets/screenshot-main.png

# Navigate down
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms

# Change theme
Type "t"
Sleep 300ms
Type "t"
Sleep 300ms
Screenshot assets/screenshot-theme.png

# Show help
Type "?"
Sleep 600ms
Screenshot assets/screenshot-help.png

# Close help
Type "?"
Sleep 200ms

# Show compose
Type "n"
Sleep 600ms
Screenshot assets/screenshot-compose.png

# Exit
Escape
Sleep 200ms
Type "q"
