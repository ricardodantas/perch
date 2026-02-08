# VHS tape for Perch screenshots
# Run with: vhs screenshots.tape

Output screenshots/01-timeline.png
Output screenshots/01-timeline.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set Padding 20

Type "cd /var/home/ricardodantas/dev/perch && ./target/release/perch --demo"
Enter
Sleep 2s
Screenshot screenshots/01-timeline.png

# Open help
Type "?"
Sleep 1s
Screenshot screenshots/02-help.png

# Close help
Escape
Sleep 500ms

# Open theme picker (lowercase t)
Type "t"
Sleep 1s
Screenshot screenshots/03-theme-picker.png

# Select a different theme (move down a few times)
Down
Down
Down
Sleep 500ms
Screenshot screenshots/04-theme-selection.png

# Apply theme
Enter
Sleep 500ms
Screenshot screenshots/05-nord-theme.png

# Back to Catppuccin - open theme picker again
Type "t"
Sleep 500ms
Down
Down
Down
Down
Down
Down
Down
Sleep 500ms
Enter
Sleep 500ms

# Open compose dialog
Type "n"
Sleep 1s
Screenshot screenshots/06-compose.png

# Close compose
Escape
Sleep 500ms

# Go to accounts view
Type "2"
Sleep 1s
Screenshot screenshots/07-accounts.png

# Back to timeline
Type "1"
Sleep 500ms

# Open about
Type "A"
Sleep 1s
Screenshot screenshots/08-about.png

# Close about
Escape
Sleep 500ms

# Quit
Type "q"
