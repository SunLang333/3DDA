import os
import json
import csv
import glob

def run():
    out_file = 'AA00REWORK/data/items.csv'
    os.makedirs('AA00REWORK/data', exist_ok=True)
    
    files = glob.glob('data/json/items/**/*.json', recursive=True)
    
    with open(out_file, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(['id', 'type', 'name', 'weight', 'volume'])
        
        count = 0
        for file in files:
            with open(file, 'r', encoding='utf-8') as jf:
                try:
                    data = json.load(jf)
                    for item in data:
                        if isinstance(item, dict) and 'id' in item and 'type' in item:
                            name = item.get('name', item.get('id', ''))
                            if isinstance(name, dict): name = name.get('str', item.get('id', ''))
                            elif isinstance(name, list): name = name[0]
                            weight = item.get('weight', '')
                            volume = item.get('volume', '')
                            writer.writerow([item['id'], item['type'], name, weight, volume])
                            count += 1
                except Exception as e:
                    pass
    print(f'Wrote {count} items to {out_file}')

run()
