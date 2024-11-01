
SRCS=$(wildcard *.v)
FAMILY=xcup
# flatten design before synthesis, no clock buffers, no IO buffers, no carry logic, no MUXes
SYNTH_OPT=-flatten -noclkbuf -noiopad -nocarry -nowidelut -ise
YOSYS=yosys # Yosys 0.33 (git sha1 2584903a060)
FSTEM=synth

.PHONY: all clean

all: mux_4_1_synth.v

clean:
	rm -f *_$(FSTEM).v *.ys *.dot *.png

%_$(FSTEM).v: %_$(FSTEM).ys $(SRCS)
	+$(YOSYS) -s $<
	# Script must have made a dot file
	dot -Tpng $*_$(FSTEM).dot > $*_$(FSTEM).png

%_$(FSTEM).ys: %.v
	@echo "read_verilog $(SRCS)" > $@
	@echo "synth_xilinx -top $* -family $(FAMILY) $(SYNTH_OPT)" >> $@
	@echo "clean -purge" >> $@
	@echo "write_verilog $*_$(FSTEM).v" >> $@
	# Make graph as bonus
	@echo "show -viewer none -prefix $*_$(FSTEM)" >> $@
