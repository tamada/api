#!/usr/bin/env python3
import json
import os
import sys
import argparse
from pathlib import Path

def test_json_validity(path):
    try:
        with open(path, 'r', encoding='utf-8') as f:
            json.load(f)
        return True
    except Exception as e:
        print(f"Error: {path} is not a valid JSON file: {e}")
        return False

def check_privacy_members(api_dir):
    members_dir = api_dir / 'members'
    if not members_dir.exists():
        return True
    
    passed = True
    # Check all files in members directory and its subdirectories
    for p in members_dir.rglob('*'):
        if p.is_file():
            try:
                with open(p, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                
                # If it's a list (like members/all)
                if isinstance(data, list):
                    for item in data:
                        if 'student_id' in item:
                            print(f"Privacy Leak: 'student_id' found in {p}")
                            passed = False
                # If it's an object (like members/ids/00-tamada)
                elif isinstance(data, dict):
                    if 'student_id' in data:
                        print(f"Privacy Leak: 'student_id' found in {p}")
                        passed = False
            except json.JSONDecodeError:
                continue # Skip non-json files if any
    return passed

def check_privacy_grants(api_dir):
    grants_dir = api_dir / 'grants'
    if not grants_dir.exists():
        return True
    
    passed = True
    for p in grants_dir.rglob('*'):
        if p.is_file():
            try:
                with open(p, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                
                if isinstance(data, list):
                    for item in data:
                        if item.get('secret') is True:
                            print(f"Privacy Leak: Secret grant found in {p}: {item.get('id')}")
                            passed = False
            except json.JSONDecodeError:
                continue
    return passed

def check_structure(api_dir, expected_files):
    passed = True
    for rel_path in expected_files:
        p = api_dir / rel_path
        if not p.exists():
            print(f"Missing File: Expected file {rel_path} not found.")
            passed = False
        elif not test_json_validity(p):
            passed = False
    return passed

def main():
    parser = argparse.ArgumentParser(description='Test API deployment for privacy and structure.')
    parser.add_argument('--tamada-api', help='Path to tamada/api directory')
    parser.add_argument('--tamadalab-api', help='Path to tamadalab/api directory')
    args = parser.parse_args()

    overall_passed = True

    if args.tamada_api:
        print(f"Testing Tamada API at {args.tamada_api}...")
        api_dir = Path(args.tamada_api)
        expected = ['profile', 'activities', 'degrees', 'skills', 'job-histories/index.json', 'index.json', 'ja']
        if not check_structure(api_dir, expected): overall_passed = False
        print("Tamada API structure check complete.")

    if args.tamadalab_api:
        print(f"Testing TamadaLab API at {args.tamadalab_api}...")
        api_dir = Path(args.tamadalab_api)
        expected = ['members/all', 'papers/index.json', 'theses/index.json', 'grants/index.json', 'index.json', 'ja']
        if not check_structure(api_dir, expected): overall_passed = False
        
        if not check_privacy_members(api_dir): overall_passed = False
        if not check_privacy_grants(api_dir): overall_passed = False
        print("TamadaLab API privacy and structure checks complete.")

    if overall_passed:
        print("\nALL TESTS PASSED")
        sys.exit(0)
    else:
        print("\nTESTS FAILED")
        sys.exit(1)

if __name__ == '__main__':
    main()
