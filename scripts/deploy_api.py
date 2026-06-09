#!/usr/bin/env python3
import json
import os
import argparse
from pathlib import Path

def save_json(data, path, extensionless=True):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

def sanitize_filename(name):
    return name.replace('/', '-')

def process_personal(source_dir, api_dir):
    # profile
    p = source_dir / 'profile.json'
    if p.exists():
        with open(p) as f:
            save_json(json.load(f), api_dir / 'profile')
    
    # activities
    p = source_dir / 'activities.json'
    if p.exists():
        with open(p) as f:
            save_json(json.load(f), api_dir / 'activities')

    # degrees
    p = source_dir / 'degrees.json'
    if p.exists():
        with open(p) as f:
            save_json(json.load(f), api_dir / 'degrees')

    # skills
    p = source_dir / 'skills.json'
    if p.exists():
        with open(p) as f:
            save_json(json.load(f), api_dir / 'skills')

    # job-histories
    p = source_dir / 'job-histories.json'
    if p.exists():
        with open(p) as f:
            items = json.load(f)
            save_json(items, api_dir / 'job-histories' / 'index.json', extensionless=False)
            if items:
                save_json(items[0], api_dir / 'job-histories' / 'current')
            
            # Group by 'as'
            as_groups = {}
            for item in items:
                as_val = item.get('as')
                if as_val:
                    if as_val not in as_groups:
                        as_groups[as_val] = []
                    as_groups[as_val].append(item)
            for as_val, group in as_groups.items():
                filename = sanitize_filename(as_val)
                save_json(group, api_dir / 'job-histories' / filename)

def process_lab(source_dir, api_dir):
    # members
    p = source_dir / 'members.json'
    if p.exists():
        with open(p) as f:
            data = json.load(f)
            items = data.get('items', [])
            save_json(items, api_dir / 'members' / 'all')
            
            # category
            cats = {}
            for item in items:
                c = item.get('category')
                if c:
                    if c not in cats:
                        cats[c] = []
                    cats[c].append(item)
            for c, group in cats.items():
                filename = sanitize_filename(c)
                save_json(group, api_dir / 'members' / filename)
            
            # ids
            for item in items:
                id_val = item.get('id')
                if id_val:
                    # Remove student_id for privacy as per spec
                    item_copy = item.copy()
                    if 'student_id' in item_copy:
                        del item_copy['student_id']
                    filename = sanitize_filename(id_val)
                    save_json(item_copy, api_dir / 'members' / 'ids' / filename)

    # papers
    p = source_dir / 'papers.json'
    if p.exists():
        with open(p) as f:
            data = json.load(f)
            items = data.get('items', [])
            save_json(items, api_dir / 'papers' / 'index.json', extensionless=False)
            save_json(items[:50], api_dir / 'papers' / 'recent')
            
            # years
            years = {}
            # types
            types_map = {}
            # languages
            langs = {'japanese': [], 'english': []}
            
            for item in items:
                y = str(item.get('year'))
                if y:
                    if y not in years:
                        years[y] = []
                    years[y].append(item)
                
                ts = item.get('types', [])
                for t in ts:
                    if t == 'japanese':
                        langs['japanese'].append(item)
                    elif t == 'english':
                        langs['english'].append(item)
                    else:
                        if t not in types_map:
                            types_map[t] = []
                        types_map[t].append(item)
                
                # ids
                id_val = item.get('id')
                if id_val:
                    filename = sanitize_filename(id_val)
                    save_json(item, api_dir / 'papers' / 'ids' / filename)
            
            for y, group in years.items():
                filename = sanitize_filename(y)
                save_json(group, api_dir / 'papers' / 'years' / filename)
            for t, group in types_map.items():
                filename = sanitize_filename(t)
                save_json(group, api_dir / 'papers' / 'types' / filename)
            save_json(langs['japanese'], api_dir / 'papers' / 'languages' / 'japanese')
            save_json(langs['english'], api_dir / 'papers' / 'languages' / 'english')

    # theses
    p = source_dir / 'theses.json'
    if p.exists():
        with open(p) as f:
            data = json.load(f)
            items = data.get('items', [])
            save_json(items, api_dir / 'theses' / 'index.json', extensionless=False)
            
            years = {}
            for item in items:
                y = str(item.get('year'))
                if y:
                    if y not in years:
                        years[y] = []
                    years[y].append(item)
            for y, group in years.items():
                filename = sanitize_filename(y)
                save_json(group, api_dir / 'theses' / 'years' / filename)

    # grants
    p = source_dir / 'grants.json'
    if p.exists():
        with open(p) as f:
            items = json.load(f)
            save_json(items, api_dir / 'grants' / 'index.json', extensionless=False)
            actives = [i for i in items if i.get('status') == 'active']
            save_json(actives, api_dir / 'grants' / 'actives')

    # activities (Lab)
    p = source_dir / 'activities.json'
    if p.exists():
        with open(p) as f:
            save_json(json.load(f), api_dir / 'activities')

def generate_discovery(api_dir):
    en = {
        "endpoints": [
            {"path": "/api", "description": "API Discovery (English)"},
            {"path": "/api/ja", "description": "API Discovery (Japanese)"},
            {"path": "/api/profile", "description": "Haruaki Tamada's profile"},
            {"path": "/api/activities", "description": "Conference and academic activities"},
            {"path": "/api/degrees", "description": "Academic degrees obtained"},
            {"path": "/api/job-histories", "description": "Career history"},
            {"path": "/api/job-histories/current", "description": "Current career entry"},
            {"path": "/api/skills", "description": "Technical and language skills"},
            {"path": "/api/members/all", "description": "All lab members (students, alumni, collaborators)"},
            {"path": "/api/papers", "description": "All academic publications"},
            {"path": "/api/papers/recent", "description": "Latest publications"},
            {"path": "/api/theses", "description": "All bachelor, master, and doctoral theses"},
            {"path": "/api/grants", "description": "Research grants and funding"},
            {"path": "/api/grants/actives", "description": "Active research grants"}
        ]
    }
    ja = {
        "endpoints": [
            {"path": "/api", "description": "API 探索 (英語)"},
            {"path": "/api/ja", "description": "API 探索 (日本語)"},
            {"path": "/api/profile", "description": "玉田 春昭のプロフィール情報"},
            {"path": "/api/activities", "description": "学会活動および研究室活動実績"},
            {"path": "/api/degrees", "description": "取得学位情報"},
            {"path": "/api/job-histories", "description": "職歴情報一覧"},
            {"path": "/api/job-histories/current", "description": "現在の主な職歴"},
            {"path": "/api/skills", "description": "スキル・技術スタック"},
            {"path": "/api/members/all", "description": "研究室全メンバー一覧"},
            {"path": "/api/papers", "description": "研究業績（論文・発表）一覧"},
            {"path": "/api/papers/recent", "description": "最新の研究業績"},
            {"path": "/api/theses", "description": "研究室の学位論文一覧"},
            {"path": "/api/grants", "description": "研究資金・助成金情報"},
            {"path": "/api/grants/actives", "description": "現在継続中の研究資金"}
        ]
    }
    save_json(en, api_dir / 'index.json', extensionless=False)
    save_json(ja, api_dir / 'ja')


def main():
    parser = argparse.ArgumentParser(description='Deploy JSON data to REST API structure.')
    parser.add_argument('--source-dir', required=True, help='Directory containing source JSON files.')
    parser.add_argument('--output-dir', required=True, help='Directory to output the API structure.')
    args = parser.parse_args()

    source = Path(args.source_dir)
    api_dir = Path(args.output_dir) / 'api'

    # Process Personal data if present
    personal_indicators = ['profile.json', 'job-histories.json']
    if any((source / ind).exists() for ind in personal_indicators):
        process_personal(source, api_dir)
    
    # Process Lab data if present
    lab_indicators = ['members.json', 'papers.json', 'grants.json']
    if any((source / ind).exists() for ind in lab_indicators):
        process_lab(source, api_dir)
    
    # Always generate/update discovery if we are deploying to an api dir
    generate_discovery(api_dir)

if __name__ == '__main__':
    main()
