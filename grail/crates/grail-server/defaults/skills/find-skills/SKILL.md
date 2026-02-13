---
name: find-skills
description: Helps users discover and install agent skills when they ask questions like "how do I do X", "find a skill for X", "is there a skill that can...", or express interest in extending capabilities.
---

# Find Skills

This skill helps you discover and install skills from the open agent skills ecosystem.

## When to Use This Skill

Use this skill when the user:

- Asks "how do I do X" where X might map to an existing skill
- Says "find a skill for X" or "is there a skill for X"
- Asks "can you do X" where X is a specialized capability
- Wants to extend agent capabilities
- Wants to search for tools, templates, or workflows

## Skills CLI

The Skills CLI (`npx skills`) is the package manager for skills.

Key commands:

- `npx skills find [query]`
- `npx skills add <package>`
- `npx skills check`
- `npx skills update`

Browse skills at: https://skills.sh/

## Workflow

1. Understand user intent and domain.
2. Search with `npx skills find <query>`.
3. Present relevant options with install commands and links.
4. Offer to install if requested.

Install command pattern:

```bash
npx skills add <owner/repo@skill> -g -y
```

## If Nothing Is Found

If no relevant skill exists:

1. Say no matching skill was found.
2. Offer to handle the task directly.
3. Suggest creating a new skill with `npx skills init`.

