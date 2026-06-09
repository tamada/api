# Work History - June 9, 2026

## Overview

Established an automated, privacy-conscious, and multi-repository deployment system for the REST API.

## Key Accomplishments

### 1. Refactoring and Script Development

- **`scripts/api_utils.py`**: Created a shared utility module for JSON operations and filename sanitization.
- **`scripts/translate_tamada.py`**: Developed a specialized script for personal API data (profile, job-histories, etc.).
- **`scripts/translate_tamadalab.py`**: Developed a specialized script for laboratory API data (members, papers, grants, etc.).

### 2. Privacy and Security

- **Member Data**: Automated removal of `student_id` from all API outputs.
- **Grant Data**: Implemented filtering to exclude entries marked as `secret: true` (rejected applications).
- **Verification**: Created `scripts/test_deploy.py` to automatically detect privacy leaks and structural errors before deployment.

### 3. Automation and CI/CD

- **`Justfile`**: Updated to support unified `generate` and `test` targets with configurable output directories.
- **GitHub Actions**: Created `.github/workflows/deploy.yml` to automate the build, test, and deployment process.
  - Supports dual deployment to both the root repository and the external `tamadalab/api` repository.
  - Implemented non-destructive deployment by merging new API files with existing static content on the `gh-pages` branch.

### 4. Technical Research

- Investigated Pkl language specifics, confirming that `as` and `new` are reserved keywords and documenting how to use backticks for escaping.
