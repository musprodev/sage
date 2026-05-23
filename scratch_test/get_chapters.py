import json, re

with open('novelbuddy_book.html', 'r') as f:
    html = f.read()

m = re.search(r'<script id="__NEXT_DATA__" type="application/json">(.+?)</script>', html)
if m:
    data = json.loads(m.group(1))
    props = data['props']['pageProps']
    print(list(props.keys()))
