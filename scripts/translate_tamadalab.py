#!/usr/bin/env python3
import json
import argparse
from pathlib import Path
from api_utils import save_json, sanitize_filename, generate_discovery

def translate_lab(source_dir, api_dir):
    # members
    p = source_dir / 'members.json'
    if p.exists():
        with open(p) as f:
            data = json.load(f)
            items = data.get('items', [])
            
            # Remove student_id from all members immediately
            clean_items = []
            for item in items:
                item_copy = item.copy()
                if 'student_id' in item_copy:
                    del item_copy['student_id']
                clean_items.append(item_copy)
            
            save_json(clean_items, api_dir / 'members' / 'all')
            
            cats = {}
            for item in clean_items:
                c = item.get('category')
                if c:
                    cats.setdefault(c, []).append(item)
            for c, group in cats.items():
                filename = sanitize_filename(c)
                save_json(group, api_dir / 'members' / filename)
            
            for item in clean_items:
                id_val = item.get('id')
                if id_val:
                    filename = sanitize_filename(id_val)
                    save_json(item, api_dir / 'members' / 'ids' / filename)

    # papers
    p = source_dir / 'papers.json'
    if p.exists():
        with open(p) as f:
            data = json.load(f)
            items = data.get('items', [])
            save_json(items, api_dir / 'papers' / 'index.json')
            save_json(items[:50], api_dir / 'papers' / 'recent')
            
            years, types_map = {}, {}
            langs = {'japanese': [], 'english': []}
            
            for item in items:
                y = str(item.get('year'))
                if y:
                    years.setdefault(y, []).append(item)
                
                for t in item.get('types', []):
                    if t == 'japanese': langs['japanese'].append(item)
                    elif t == 'english': langs['english'].append(item)
                    else: types_map.setdefault(t, []).append(item)
                
                id_val = item.get('id')
                if id_val:
                    save_json(item, api_dir / 'papers' / 'ids' / sanitize_filename(id_val))
            
            for y, group in years.items():
                save_json(group, api_dir / 'papers' / 'years' / sanitize_filename(y))
            for t, group in types_map.items():
                save_json(group, api_dir / 'papers' / 'types' / sanitize_filename(t))
            save_json(langs['japanese'], api_dir / 'papers' / 'languages' / 'japanese')
            save_json(langs['english'], api_dir / 'papers' / 'languages' / 'english')

    # theses
    p = source_dir / 'theses.json'
    if p.exists():
        with open(p) as f:
            data = json.load(f)
            items = data.get('items', [])
            save_json(items, api_dir / 'theses' / 'index.json')
            years = {}
            for item in items:
                y = str(item.get('year'))
                if y: years.setdefault(y, []).append(item)
            for y, group in years.items():
                save_json(group, api_dir / 'theses' / 'years' / sanitize_filename(y))

    # grants
    p = source_dir / 'grants.json'
    if p.exists():
        with open(p) as f:
            items = json.load(f)
            # Filter out secret grants (not accepted)
            public_items = [i for i in items if not i.get('secret', False)]
            save_json(public_items, api_dir / 'grants' / 'index.json')
            actives = [i for i in public_items if i.get('status') == 'active']
            save_json(actives, api_dir / 'grants' / 'actives')

    # activities (Lab)
    p = source_dir / 'activities.json'
    if p.exists():
        with open(p) as f:
            save_json(json.load(f), api_dir / 'activities')

def main():
    parser = argparse.ArgumentParser(description='Translate lab JSON data to API structure.')
    parser.add_argument('--source-dir', required=True, help='Source directory for JSON files.')
    parser.add_argument('--output-dir', required=True, help='Output directory for the API.')
    args = parser.parse_args()

    source = Path(args.source_dir)
    api_dir = Path(args.output_dir)

    translate_lab(source, api_dir)

    en = [
        {"path": "/api/members/all", "description": "All lab members"},
        {"path": "/api/papers", "description": "Research publications"},
        {"path": "/api/theses", "description": "Graduation theses"},
        {"path": "/api/grants", "description": "Research grants"},
        {"path": "/api/activities", "description": "Laboratory activities"}
    ]
    ja = [
        {"path": "/api/members/all", "description": "研究室全メンバー一覧"},
        {"path": "/api/papers", "description": "研究業績（論文・発表）一覧"},
        {"path": "/api/theses", "description": "研究室の学位論文一覧"},
        {"path": "/api/grants", "description": "研究資金情報"},
        {"path": "/api/activities", "description": "研究室活動実績"}
    ]
    generate_discovery(api_dir, en, ja)

if __name__ == '__main__':
    main()
