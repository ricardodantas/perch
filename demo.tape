# VHS tape for Perch screenshots
Output screenshots/01-timeline.gif

Set FontFamily "JetBrains Mono"
Set FontSize 14
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"

Type "./target/release/perch --demo"
Enter
Sleep 1500ms

# Take main screenshot
Screenshot screenshots/01-timeline.png

# Navigate down to show selection
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms

# Show post detail
Type "l"
Sleep 500ms
Screenshot screenshots/02-detail.png

# Go back to timeline
Type "h"
Sleep 200ms

# Show help
Type "?"
Sleep 600ms
Screenshot screenshots/02-help.png

# Close help
Type "?"
Sleep 200ms

# Change theme
Type "t"
Sleep 300ms
Screenshot screenshots/03-theme-picker.png

# Navigate to Nord theme
Type "j"
Sleep 100ms
Type "j"
Sleep 100ms
Type "j"
Sleep 100ms
Type "j"
Sleep 100ms
Screenshot screenshots/04-theme-selection.png

# Select Nord
Enter
Sleep 300ms
Screenshot screenshots/05-nord-theme.png

# Show compose
Type "n"
Sleep 600ms
Screenshot screenshots/06-compose.png

# Exit compose
Escape
Sleep 200ms

# Show accounts view
Type "2"
Sleep 300ms
Screenshot screenshots/07-accounts.png

# Back to timeline, then show about
Type "1"
Sleep 200ms
Type "A"
Sleep 300ms
Screenshot screenshots/08-about.png

# Exit
Escape
Sleep 200ms
Type "q"
