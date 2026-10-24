#!/bin/sh
# Regenerates the QPTIFF references (Akoya samples from the OME server, CC BY 4.0).
#
# Usage: REP=3 sh tests/scripts/oracle_qptiff.sh
set -e
cd "$(dirname "$0")/../.."
Q=tests/reference/ome/Vectra-QPTIFF/perkinelmer
REP=${REP:-1}
python3 -I tests/scripts/oracle.py qptiff --repeat $REP --keep-series 0 "$Q/PKI_fields/HnE_3_1x1component_data.tif"
python3 -I tests/scripts/oracle.py qptiff --repeat 1 --series 1 --ours-series assoc:thumbnail --append "$Q/PKI_fields/HnE_3_1x1component_data.tif"
python3 -I tests/scripts/oracle.py qptiff --repeat $REP --keep-series 0 --append "$Q/PKI_fields/LuCa-7color_[13860,52919]_1x1component_data.tif"
python3 -I tests/scripts/oracle.py qptiff --repeat $REP --series 0 --crop 20000,15000,2048,2048 --append "$Q/PKI_scans/HandEcompressed_Scan1.qptiff"
python3 -I tests/scripts/oracle.py qptiff --repeat $REP --series 0 --crop 20000,15000,2048,2048 --append "$Q/PKI_scans/LuCa-7color_Scan1.qptiff"
python3 -I tests/scripts/oracle.py qptiff --repeat 1 --series 3 --ours-series 0/3 --append "$Q/PKI_scans/LuCa-7color_Scan1.qptiff"
python3 -I tests/scripts/oracle.py qptiff --repeat 1 --series 7 --ours-series assoc:macro --append "$Q/PKI_scans/LuCa-7color_Scan1.qptiff"
python3 -I tests/scripts/oracle.py qptiff --repeat 1 --series 8 --ours-series assoc:label --append "$Q/PKI_scans/LuCa-7color_Scan1.qptiff"
python3 -I tests/scripts/oracle.py qptiff --repeat 1 --series 3 --ours-series 0/3 --append "$Q/PKI_scans/HandEcompressed_Scan1.qptiff"
