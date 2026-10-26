#!/usr/bin/env python3
"""核验真实交付的 8 张电子 PNG、双能窗、费米线及插值保留色。"""
import argparse, json
from pathlib import Path
from PIL import Image
p=argparse.ArgumentParser();p.add_argument("results",type=Path);a=p.parse_args()
files=sorted(a.results.glob("3.qe-electronic/*/electronic-*.png"))+sorted(a.results.glob("5.epw-wannier/electronic-*.png"))
assert len(files)==8,files
records=[]
for file in files:
    image=Image.open(file).convert("RGB")
    info=Image.open(file).info
    assert image.size==(3000,2100),(file,image.size)
    assert min(info["dpi"])>=300,(file,info)
    data=json.loads(file.with_suffix(".plot.json").read_text())
    bounds=data["bounds"]
    assert bounds[0]<0<bounds[1]
    if "valence" in file.name:assert bounds[1]==5
    elif "5.epw-wannier" in str(file):
        windows={name:energy for energy,name,_ in data["window"]}
        assert bounds==[windows["dis_win_min"],windows["dis_win_max"]],(file,bounds,windows)
    else:assert bounds==[-5,5]
    red=blue=0
    for r,g,b in image.getdata():
        red+=r>200 and g<30 and b<30
        blue+=r<30 and g<110 and b>180
    assert red>1000,(file,"缺红色费米线",red)
    if "5.epw-wannier" in str(file):assert blue>500,(file,"缺蓝色插值",blue)
    else:assert blue==0,(file,"QE 图占用蓝色",blue)
    records.append({"file":str(file.relative_to(a.results)),"bounds_ev":bounds,"pixels":image.size,"dpi":info["dpi"],"red_pixels":red,"blue_pixels":blue})
full=[r["bounds_ev"] for r in records if "valence" in r["file"]]
assert all(b==full[0] for b in full)
pdos=json.loads((a.results/"3.qe-electronic/qe_eledos/electronic-pdos.json").read_text())
assert pdos["complete"] and set(pdos["elements"])=={"Mg","B"} and not pdos["rescale"]
print(json.dumps({"figures":records,"pdos":pdos,"logical_figure_count":8},ensure_ascii=False,indent=2))
