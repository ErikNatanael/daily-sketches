## Using ffmpeg to create a video file from frames

mactuitui:
ffmpeg -r 60 -i frame-%04d.png -pix_fmt yuv420p -s 1024x1024 200314.mp4

Alt, Wilgre:
ffmpeg -framerate 30 -i %d.png -pix_fmt yuv420p -vcodec libx264 -preset veryslow -c:a aac video.mp4

Finjusterad lite:
ffmpeg -r 60 -start_number 0 -i %d.png -pix_fmt yuv420p -c:v libx264 -crf 20 -preset veryslow -s 1024x1024 200321_crf_20_60fps_1024.mp4

ffmpeg -r 30 -start_number 0 -i %d.png -pix_fmt yuv420p -c:v libx264 -crf 19 -preset veryslow -s 1024x1024 200323_crf_20_30fps_1024.mp4

ffmpeg -r 60 -start_number 0 -i %d.png -c:v libx264 -crf 20 -preset veryslow -s 1024x1024 200326_crf_20_60fps_1024.mp4

## imagemagick to create frame grid

montage {1..64}.png -tile 8x -geometry 256x256 out.jpg

This seems buggy with many frames in a grid, sometimes it works and sometimes it doesn't. A workaround is stitching together many grid of a few frames at a time.