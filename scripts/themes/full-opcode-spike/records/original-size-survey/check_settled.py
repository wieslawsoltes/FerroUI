import os,subprocess,sys
from pathlib import Path
out=Path(__file__).resolve().parent
for name in sys.argv[1:]:
    folder=out/(name+'-settled')
    args=dict(FERROUI_CATALOG_THEME_VARIANT='light',FERROUI_SMOKE_WINDOW_SIZE='950x680',FERROUI_SMOKE_SCREENSHOTS=str(folder),FERROUI_SMOKE_SCREENSHOT_PAGES='Home,Buttons,TextBlock,ListBox,Image,Calendar,Slider,TextBox,Dialogs,TreeDataGrid',FERROUI_SMOKE_EXTEND_CLIENT_AREA='0',FERROUI_SMOKE_PAGES='2200')
    with (out/(name+'-settled.log')).open('w') as log:
        result=subprocess.run([out/name],env=dict(os.environ,**args),stdout=log,stderr=subprocess.STDOUT,timeout=55)
    frames=[f for f in folder.glob('*.png') if not f.stem.endswith(('-composed','-window'))]
    print(name,result.returncode,len(frames),[f.name for f in sorted(frames)])
