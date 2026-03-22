# GitHub Wiki Source

This directory contains GitHub Wiki page content for `Classacre/ferrumyx`.

Publish target: `https://github.com/Classacre/ferrumyx/wiki`

## Publish steps

```bash
git clone https://github.com/Classacre/ferrumyx.wiki.git
cp -r github-wiki/* ferrumyx.wiki/
cd ferrumyx.wiki
git add .
git commit -m "Update wiki"
git push
```

If your platform does not support `cp -r`, copy files manually.
