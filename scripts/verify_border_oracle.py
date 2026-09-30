"""Compare authored probes with local MCP919 WorldBorder; publish no game files."""
import argparse
import os
from pathlib import Path
import subprocess

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--java-home',type=Path,required=True)
parser.add_argument('--cargo',default='cargo')
args=parser.parse_args()
root=Path(__file__).resolve().parent.parent
mcp=root/'MCP-919'
classes=mcp/'bin/minecraft'
libraries=sorted((mcp/'jars/libraries').rglob('*.jar'))
classpath=os.pathsep.join(os.path.relpath(p,root) for p in [classes,mcp/'jars/versions/1.8.9/1.8.9.jar',*libraries])
suffix='.exe' if os.name=='nt' else ''
subprocess.run([str(args.java_home/'bin'/('javac'+suffix)),'-encoding','UTF-8','-source','8','-target','8','-cp',classpath,'-d',os.path.relpath(classes,root),'crates/rmc-java/templates/RmcBorderOracle.java'],cwd=root,check=True)
result=subprocess.run([str(args.java_home/'bin'/('java'+suffix)),'-cp',classpath,'RmcBorderOracle'],cwd=root,check=True,capture_output=True,text=True)
subprocess.run([args.cargo,'run','-p','rmc-world','--example','border_oracle_check','--offline','--locked'],cwd=root,input=result.stdout,text=True,check=True)
