import re

with open('novelfire_book.html', 'r') as f:
    html = f.read()

links = re.findall(r'href="([^"]+)"', html)
chapters = [l for l in links if 'chapter' in l.lower()]
print(chapters[:20])
