"""Stitch the view-mode captures (4 wheel notches = 480 px apart, verified by pixel match) into doc-tall.png per theme."""
from PIL import Image
names = ['01-view-top', '02-view-tables', '03-view-tasks', '04-view-latex', '04b-view-images', '05-view-mermaid', '06-view-alerts']
offs = [0, 480, 960, 1440, 1920, 2400, 2880]
X0, X1, Y0, Y1 = 323, 1895, 101, 965
SKIP = 334  # keep the earlier capture for the overlap (later ones can drop off-screen images); only its last 50 rows (edge shading) are replaced
for theme in ['github-light', 'github-dark']:
    c = Image.new('RGB', (X1 - X0, offs[-1] + (Y1 - Y0)))
    for i, (n, o) in enumerate(zip(names, offs)):
        im = Image.open(f'captures/{theme}/{n}.png').convert('RGB').crop((X0, Y0, X1, Y1))
        k = SKIP if i else 0
        c.paste(im.crop((0, k, im.size[0], im.size[1])), (0, o + k))
    c.save(f'captures/{theme}/doc-tall.png')
    print(theme, c.size)
