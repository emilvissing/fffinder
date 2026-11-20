#!/usr/bin/env bash
# Build the demo folder the screenshots are taken in. Safe to run again.
set -euo pipefail
DEMO=/home/user/polish-tools/demo/Studio
if [ -f "$DEMO/.built" ]; then echo "$DEMO"; exit 0; fi
rm -rf /home/user/polish-tools/demo
mkdir -p "$DEMO"/{Assets,"Brand guidelines",Exports,Product,Research/interviews,Projects/atlas/src,Projects/beacon,Many,"Tax [2026]"}
cd "$DEMO"
printf 'month,income,spend\nJan,1200,800\nFeb,1400,950\nMar,1100,700\n' > Budget.csv
printf '# Roadmap\n\n## Q1\n\n- Ship the search rewrite\n- Fix the sidebar\n\n## Q2\n\n- Tabs\n- Split view\n' > Roadmap.md
printf 'plain notes about the roadmap\nremember the budget review\nTODO: call the bank\n' > notes.txt
printf '# Studio\n\nEverything for the studio lives here.\n' > README.md
printf 'lowercase readme for the build scripts\n' > readme.txt
printf 'TODO list\n\n- taxes\n- invoices\n' > TODO.md
printf 'object' > TaxExemption.object
printf 'hidden' > .secret
printf '{\n  "name": "spec",\n  "version": 3,\n  "owners": ["ana", "bo"],\n  "limits": {"max": 10, "min": 1}\n}\n' > Product/spec.json
printf '# Product roadmap\n\nThe roadmap for the product team.\n' > Product/Roadmap.md
printf '[plan]\nname = "pro"\nprice = 12\n\n[plan.limits]\nseats = 5\n' > Product/pricing.toml
printf '# Typography\n\nUse Inter for text.\n' > "Brand guidelines/Typography.md"
printf '{"primary": "#0a84ff", "accent": "#ff9f0a"}\n' > "Brand guidelines/colors.json"
printf 'quarter,revenue\nQ1,10\n' > Exports/report-2026-q1.csv
printf 'quarter,revenue\nQ2,14\n' > Exports/report-2026-q2.csv
printf 'atlas project notes\nthe budget for atlas is in Budget.csv\n' > "Projects/atlas/Project Atlas.txt"
printf '# Atlas\n\nA mapping project.\n' > Projects/atlas/README.md
printf 'fn main() {\n    println!("atlas");\n}\n' > Projects/atlas/src/main.rs
printf 'pub fn budget() -> u32 {\n    // TODO: read the real budget\n    42\n}\n' > Projects/atlas/src/lib.rs
printf '# Beacon\n\nA lighthouse simulator.\n' > Projects/beacon/README.md
printf '{"name": "beacon", "version": "1.0.0"}\n' > Projects/beacon/package.json
printf 'console.log("beacon");\n' > Projects/beacon/index.js
printf '# Interview 1\n\nThey want a faster search.\n' > Research/interviews/notes-01.md
printf '# Interview 2\n\nThey want tabs.\n' > Research/interviews/notes-02.md
printf 'q,a\n1,yes\n' > "Research/survey [draft].csv"
printf '# Market analysis\n\nThe market is large.\n' > "Research/Market analysis.md"
printf 'form 1040 notes\n' > "Tax [2026]/Tax Exemption Certificate.txt"
printf 'receipts\n' > "Tax [2026]/receipts.txt"
for i in $(seq -w 1 120); do printf 'file %s\n' "$i" > "Many/file-$i.txt"; done
python3 - <<'PY'
from PIL import Image, ImageDraw
import os
d="/home/user/polish-tools/demo/Studio/Assets"
def grad(name,size,a,b):
    w,h=size; im=Image.new("RGB",size); px=im.load()
    for y in range(h):
        for x in range(w):
            t=(x+y)/(w+h)
            px[x,y]=tuple(int(a[i]*(1-t)+b[i]*t) for i in range(3))
    dr=ImageDraw.Draw(im); dr.ellipse([w*0.3,h*0.3,w*0.7,h*0.7],outline=(255,255,255),width=max(2,w//80))
    im.save(os.path.join(d,name))
grad("hero.png",(640,360),(10,132,255),(255,159,10))
grad("logo.png",(256,256),(48,209,88),(10,132,255))
grad("photo 1.jpg",(480,320),(255,55,95),(94,92,230))
grad("photo 2.jpg",(320,480),(255,214,10),(255,55,95))
grad("tiny.png",(24,24),(0,0,0),(255,255,255))
open(os.path.join(d,"icon.svg"),"w").write('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><circle cx="32" cy="32" r="28" fill="#0a84ff"/></svg>\n')
PY
(cd Exports && python3 -c "
import zipfile
z=zipfile.ZipFile('Archive.zip','w'); z.writestr('a.txt','a'); z.writestr('dir/b.txt','b'); z.close()")
touch "$DEMO/.built"
echo "$DEMO"
