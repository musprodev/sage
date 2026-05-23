import json, re

with open('novelbuddy_book.html', 'r') as f:
    html = f.read()

m = re.search(r'<script id="__NEXT_DATA__" type="application/json">(.+?)</script>', html)
if m:
    data = json.loads(m.group(1))
    props = data['props']['pageProps']
    if 'initialManga' in props:
        manga = props['initialManga']
        print("initialManga keys:", list(manga.keys()))
        if 'id' in manga:
            print("Manga id:", manga['id'])
        if 'chapters' in manga:
            print("Chapters:", len(manga['chapters']))
            print("First chapter:", manga['chapters'][0])
