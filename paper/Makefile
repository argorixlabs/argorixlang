.PHONY: analyze verify figures tables paper qa test clean all

PS := powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build_paper.ps1

analyze:
	$(PS) -Target analyze
verify:
	$(PS) -Target verify
figures:
	$(PS) -Target figures
tables:
	$(PS) -Target tables
paper:
	$(PS) -Target paper
qa:
	$(PS) -Target qa
test:
	$(PS) -Target test
clean:
	$(PS) -Target clean
all:
	$(PS) -Target all
