SRCS=$(wildcard *.v)
FAMILY=xcup
# flatten design before synthesis, no clock buffers, no IO buffers, no carry logic, no MUXes
SYNTH_OPT=-flatten -noclkbuf -noiopad -nocarry -nowidelut -ise
YOSYS=yosys # Yosys 0.33 (git sha1 2584903a060)

.PHONY: all clean

all: mux_4_1_synth.v

clean:
	rm -f *.xil *.synth *.ys *.dot *.png

%.v.synth: %_synth.ys
	+$(YOSYS) -s $<

%.v.xil: %_xil.ys
	+$(YOSYS) -s $<

# This script synthesizes to LUTs
%_xil.ys: %.v
	@echo "read_verilog $<" > $@
	@echo "synth_xilinx -family $(FAMILY) $(SYNTH_OPT)" >> $@
	@echo "clean -purge" >> $@
	@echo "splitnets -format _" >> $@
	@echo "write_verilog -simple-lhs $<.xil" >> $@

# This script synthesizes to AND, NOR, XOR, INV, MUXes
%_synth.ys: %.v
	@echo "read_verilog $<" > $@
	@echo "techmap -map lutlang.v" >> $@
	@echo "clean -purge" >> $@
	@echo "splitnets -format _" >> $@
	@echo "write_verilog -simple-lhs $<.synth" >> $@
