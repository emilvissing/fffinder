#!/usr/bin/env bash
# Rebuild the progress page from the pipeline's records. Publish it afterwards with the
# Artifact tool: file_path <out>/index.html, files {"data.json": "<out>/data.json"} (same URL).
out=/tmp/claude-0/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/scratchpad/polish-log
python3 /home/user/polish-tools/report/build.py "$out"
