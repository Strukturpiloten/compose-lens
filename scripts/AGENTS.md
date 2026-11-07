# Script guidance

Shared orchestration belongs to the selected BoxFerry interface; keep consumer wrappers thin.
Use the shared locked uv/Ruff/ty environment and install-host-tools.sh for ordinary tooling.
install-file-tools.sh is the retained image-only installer. Preserve provider/bootstrap and
shell-dispatch regressions. Image qualification is a separate GitHub CI requirement.
