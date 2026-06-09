import json
import os
from pathlib import Path

def save_json(data, path):
    """Saves data as a JSON file, ensuring the directory exists."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

def sanitize_filename(name):
    """Sanitizes strings for use as filenames (e.g., replacing '/' with '-')."""
    return name.replace('/', '-')

def generate_discovery(api_dir, endpoints_en, endpoints_ja):
    """Generates the discovery endpoints (/api and /api/ja)."""
    en = {
        "endpoints": [
            {"path": "/api", "description": "API Discovery (English)"},
            {"path": "/api/ja", "description": "API Discovery (Japanese)"}
        ] + endpoints_en
    }
    ja = {
        "endpoints": [
            {"path": "/api", "description": "API 探索 (英語)"},
            {"path": "/api/ja", "description": "API 探索 (日本語)"}
        ] + endpoints_ja
    }
    save_json(en, api_dir / 'index.json')
    save_json(ja, api_dir / 'ja')
