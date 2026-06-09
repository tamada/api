#!/usr/bin/env python3
import json
import argparse
from pathlib import Path
from api_utils import save_json, sanitize_filename, generate_discovery

def translate_personal(source_dir, api_dir):
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
            save_json(items, api_dir / 'job-histories' / 'index.json')
            if items:
                save_json(items[0], api_dir / 'job-histories' / 'current')
            
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

def main():
    parser = argparse.ArgumentParser(description='Translate personal JSON data to API structure.')
    parser.add_argument('--source-dir', required=True, help='Source directory for JSON files.')
    parser.add_argument('--output-dir', required=True, help='Output directory for the API.')
    args = parser.parse_args()

    source = Path(args.source_dir)
    api_dir = Path(args.output_dir)

    translate_personal(source, api_dir)

    en = [
        {"path": "/api/profile", "description": "Haruaki Tamada's profile"},
        {"path": "/api/activities", "description": "Academic activities"},
        {"path": "/api/degrees", "description": "Degrees obtained"},
        {"path": "/api/job-histories", "description": "Career history"},
        {"path": "/api/job-histories/current", "description": "Current position"},
        {"path": "/api/skills", "description": "Skills and technical stack"}
    ]
    ja = [
        {"path": "/api/profile", "description": "玉田 春昭のプロフィール情報"},
        {"path": "/api/activities", "description": "学会活動等実績"},
        {"path": "/api/degrees", "description": "学位取得情報"},
        {"path": "/api/job-histories", "description": "職歴情報"},
        {"path": "/api/job-histories/current", "description": "現在の役職"},
        {"path": "/api/skills", "description": "スキル・技術スタック"}
    ]
    generate_discovery(api_dir, en, ja)

if __name__ == '__main__':
    main()
