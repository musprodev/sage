import json, re, sys

with open('novelbuddy_book.html', 'r') as f:
    html = f.read()

m = re.search(r'<script id="__NEXT_DATA__" type="application/json">(.+?)</script>', html)
if m:
    data = json.loads(m.group(1))
    # print keys
    try:
        props = data['props']['pageProps']
        print(list(props.keys()))
        if 'chapters' in props:
            print("Found 'chapters' directly!")
        elif 'ssrItems' in props:
            print("Found 'ssrItems'!")
        elif 'novel' in props:
            novel = props['novel']
            print("Novel keys:", list(novel.keys()))
            if 'chapters' in novel:
                print("Found 'chapters' in 'novel'!")
        elif 'item' in props:
            item = props['item']
            print("Item keys:", list(item.keys()))
            if 'chapters' in item:
                print("Found 'chapters' in 'item'!")
        else:
            print("Need to search deeply...")
    except KeyError as e:
        print("KeyError", e)
else:
    print("No next data found")
