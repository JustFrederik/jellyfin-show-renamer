# Jellyfin Show Renamer
Example:
`renamer` or `renamer default` or `renamer dash`

allowed args:
- dash: " - {int}"
- default: "S{int}E{int}"

File structure:
`input/SeriesName/videoS01E02.mkv`

.env file for further configuration
```.dotenv
RENAMER_INPUT_FOLDER=./input
RENAMER_OUTPUT_FOLDER=./output
```