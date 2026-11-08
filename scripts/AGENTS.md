# Script guidance

Shared orchestration belongs to the selected BoxFerry interface; keep consumer wrappers thin.
Local, CI and image tools use install-host-tools.sh and its canonical integrity inventory.
install-file-tools.sh only forwards compatibility calls; do not add versions or downloaders.
Preserve provider/bootstrap and shell-dispatch regressions. Image qualification remains a
separate GitHub CI requirement.
