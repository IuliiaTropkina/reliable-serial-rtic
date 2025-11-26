# Git fetch

This memo is included as a backup plan in case something goes wrong during project work deployment.

**In case** you are **asked** to update your repo from the course upstream, please do the following:

```bash
git remote add course_upstream https://course-gitlab.tuni.fi/comp-ce-340-dependable-embedded-systems_fall2024/group_template_project.git
git fetch course_upstream
git merge course_upstream/main
```

Regarding git tagging, a good guide to follow is this one: <https://git-scm.com/book/en/v2/Git-Basics-Tagging>
