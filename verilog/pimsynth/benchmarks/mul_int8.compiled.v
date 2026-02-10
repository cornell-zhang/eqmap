/////////////////////////////////////////////////////////////
// Created by: Synopsys DC Expert(TM) in wire load mode
// Version   : S-2021.06-SP4
// Date      : Thu May 22 13:42:38 2025
/////////////////////////////////////////////////////////////

module GTECH_NOR2 (
    input A,
    input B,
    output Z
);
    assign Z = ~(A | B);
endmodule

module GTECH_NOT (
    input A,
    output Z
);
    assign Z = ~A;
endmodule

module mul_int8 ( A, B, P );
  input [7:0] A;
  input [7:0] B;
  output [7:0] P;
  wire   n1, n2, n3, n4, n5, n6, n7, n8, n9, n10, n11, n12, n13, n14, n15, n16,
         n17, n18, n19, n20, n21, n22, n23, n24, n25, n26, n27, n28, n29, n30,
         n31, n32, n33, n34, n35, n36, n37, n38, n39, n40, n41, n42, n43, n44,
         n45, n46, n47, n48, n49, n50, n51, n52, n53, n54, n55, n56, n57, n58,
         n59, n60, n61, n62, n63, n64, n65, n66, n67, n68, n69, n70, n71, n72,
         n73, n74, n75, n76, n77, n78, n79, n80, n81, n82, n83, n84, n85, n86,
         n87, n88, n89, n90, n91, n92, n93, n94, n95, n96, n97, n98, n99, n100,
         n101, n102, n103, n104, n105, n106, n107, n108, n109, n110, n111,
         n112, n113, n114, n115, n116, n117, n118, n119, n120, n121, n122,
         n123, n124, n125, n126, n127, n128, n129, n130, n131, n132, n133,
         n134, n135, n136, n137, n138, n139, n140, n141, n142, n143, n144,
         n145, n146, n147, n148, n149, n150, n151, n152, n153, n154, n155,
         n156, n157, n158, n159, n160, n161, n162, n163, n164, n165, n166,
         n167, n168, n169, n170, n171, n172, n173, n174, n175, n176, n177,
         n178, n179, n180, n181, n182, n183, n184, n185, n186, n187, n188,
         n189, n190, n191, n192, n193, n194, n195, n196, n197, n198, n199,
         n200, n201, n202, n203, n204, n205, n206, n207, n208, n209, n210,
         n211, n212, n213, n214, n215, n216, n217, n218, n219, n220, n221,
         n222, n223, n224, n225, n226, n227, n228, n229, n230, n231, n232,
         n233, n234, n235, n236, n237, n238, n239, n240, n241, n242, n243,
         n244, n245, n246, n247, n248, n249, n250, n251, n252, n253, n254,
         n255, n256, n257, n258, n259, n260, n261, n262, n263, n264, n265,
         n266, n267, n268, n269, n270, n271, n272, n273, n274, n275, n276,
         n277, n278, n279, n280, n281, n282, n283, n284, n285, n286, n287,
         n288, n289, n290, n291, n292, n293, n294, n295, n296, n297, n298,
         n299, n300, n301, n302, n303, n304, n305, n306, n307, n308, n309,
         n310, n311, n312, n313, n314, n315, n316, n317, n318, n319, n320,
         n321, n322, n323, n324, n325, n326, n327, n328, n329, n330, n331,
         n332, n333, n334, n335, n336, n337, n338, n339, n340, n341, n342,
         n343, n344, n345, n346, n347, n348, n349, n350, n351, n352, n353,
         n354, n355, n356, n357, n358, n359, n360, n361, n362, n363, n364,
         n365, n366, n367, n368, n369, n370, n371, n372;

  GTECH_NOR2 U1 ( .A(n1), .B(n2), .Z(P[7]) );
  GTECH_NOR2 U2 ( .A(n3), .B(n4), .Z(n2) );
  GTECH_NOT U3 ( .A(n5), .Z(n3) );
  GTECH_NOR2 U4 ( .A(n6), .B(n5), .Z(n1) );
  GTECH_NOR2 U5 ( .A(n7), .B(n8), .Z(n5) );
  GTECH_NOR2 U6 ( .A(n9), .B(n10), .Z(n8) );
  GTECH_NOT U7 ( .A(n11), .Z(n9) );
  GTECH_NOR2 U8 ( .A(n12), .B(n11), .Z(n7) );
  GTECH_NOR2 U9 ( .A(n13), .B(n14), .Z(n11) );
  GTECH_NOR2 U10 ( .A(n15), .B(n16), .Z(n14) );
  GTECH_NOT U11 ( .A(n17), .Z(n15) );
  GTECH_NOR2 U12 ( .A(n18), .B(n17), .Z(n13) );
  GTECH_NOR2 U13 ( .A(n19), .B(n20), .Z(n17) );
  GTECH_NOR2 U14 ( .A(n21), .B(n22), .Z(n20) );
  GTECH_NOT U15 ( .A(n23), .Z(n22) );
  GTECH_NOR2 U16 ( .A(n23), .B(n24), .Z(n19) );
  GTECH_NOT U17 ( .A(n21), .Z(n24) );
  GTECH_NOR2 U18 ( .A(n25), .B(n26), .Z(n21) );
  GTECH_NOR2 U19 ( .A(n27), .B(n28), .Z(n23) );
  GTECH_NOT U20 ( .A(n16), .Z(n18) );
  GTECH_NOR2 U21 ( .A(n29), .B(n30), .Z(n16) );
  GTECH_NOR2 U22 ( .A(n31), .B(n32), .Z(n30) );
  GTECH_NOT U23 ( .A(n33), .Z(n32) );
  GTECH_NOR2 U24 ( .A(n33), .B(n34), .Z(n29) );
  GTECH_NOT U25 ( .A(n31), .Z(n34) );
  GTECH_NOR2 U26 ( .A(n35), .B(n36), .Z(n31) );
  GTECH_NOR2 U27 ( .A(n37), .B(n38), .Z(n33) );
  GTECH_NOT U28 ( .A(n10), .Z(n12) );
  GTECH_NOR2 U29 ( .A(n39), .B(n40), .Z(n10) );
  GTECH_NOR2 U30 ( .A(n41), .B(n42), .Z(n40) );
  GTECH_NOT U31 ( .A(n43), .Z(n41) );
  GTECH_NOR2 U32 ( .A(n44), .B(n43), .Z(n39) );
  GTECH_NOR2 U33 ( .A(n45), .B(n46), .Z(n43) );
  GTECH_NOR2 U34 ( .A(n47), .B(n48), .Z(n46) );
  GTECH_NOT U35 ( .A(n49), .Z(n48) );
  GTECH_NOR2 U36 ( .A(n49), .B(n50), .Z(n45) );
  GTECH_NOT U37 ( .A(n47), .Z(n50) );
  GTECH_NOR2 U38 ( .A(n51), .B(n52), .Z(n47) );
  GTECH_NOR2 U39 ( .A(n53), .B(n54), .Z(n49) );
  GTECH_NOT U40 ( .A(n42), .Z(n44) );
  GTECH_NOR2 U41 ( .A(n55), .B(n56), .Z(n42) );
  GTECH_NOR2 U42 ( .A(n57), .B(n58), .Z(n56) );
  GTECH_NOR2 U43 ( .A(n59), .B(n60), .Z(n55) );
  GTECH_NOT U44 ( .A(n57), .Z(n60) );
  GTECH_NOR2 U45 ( .A(n61), .B(n62), .Z(n57) );
  GTECH_NOR2 U46 ( .A(n63), .B(n64), .Z(n61) );
  GTECH_NOT U47 ( .A(n58), .Z(n59) );
  GTECH_NOR2 U48 ( .A(n65), .B(n66), .Z(n58) );
  GTECH_NOR2 U49 ( .A(n67), .B(n68), .Z(n66) );
  GTECH_NOT U50 ( .A(n69), .Z(n68) );
  GTECH_NOR2 U51 ( .A(n70), .B(n71), .Z(n67) );
  GTECH_NOT U52 ( .A(n72), .Z(n71) );
  GTECH_NOR2 U53 ( .A(n73), .B(n74), .Z(n72) );
  GTECH_NOR2 U54 ( .A(n75), .B(n76), .Z(n74) );
  GTECH_NOR2 U55 ( .A(n77), .B(n78), .Z(n73) );
  GTECH_NOT U56 ( .A(n79), .Z(n78) );
  GTECH_NOR2 U57 ( .A(B[2]), .B(n80), .Z(n70) );
  GTECH_NOT U58 ( .A(n81), .Z(n80) );
  GTECH_NOR2 U59 ( .A(n69), .B(n82), .Z(n65) );
  GTECH_NOR2 U60 ( .A(n83), .B(n84), .Z(n82) );
  GTECH_NOT U61 ( .A(n85), .Z(n84) );
  GTECH_NOR2 U62 ( .A(n86), .B(n87), .Z(n85) );
  GTECH_NOR2 U63 ( .A(n88), .B(n76), .Z(n87) );
  GTECH_NOT U64 ( .A(n77), .Z(n76) );
  GTECH_NOR2 U65 ( .A(n89), .B(n90), .Z(n77) );
  GTECH_NOR2 U66 ( .A(B[2]), .B(n75), .Z(n86) );
  GTECH_NOT U67 ( .A(n88), .Z(n75) );
  GTECH_NOR2 U68 ( .A(n79), .B(n91), .Z(n88) );
  GTECH_NOR2 U69 ( .A(n92), .B(n93), .Z(n91) );
  GTECH_NOR2 U70 ( .A(A[5]), .B(n79), .Z(n83) );
  GTECH_NOR2 U71 ( .A(n94), .B(n95), .Z(n69) );
  GTECH_NOT U72 ( .A(n96), .Z(n95) );
  GTECH_NOR2 U73 ( .A(n97), .B(n98), .Z(n96) );
  GTECH_NOR2 U74 ( .A(A[7]), .B(n99), .Z(n98) );
  GTECH_NOT U75 ( .A(n100), .Z(n94) );
  GTECH_NOR2 U76 ( .A(n101), .B(n102), .Z(n100) );
  GTECH_NOR2 U77 ( .A(B[0]), .B(A[6]), .Z(n102) );
  GTECH_NOR2 U78 ( .A(n92), .B(n103), .Z(n101) );
  GTECH_NOT U79 ( .A(n104), .Z(n103) );
  GTECH_NOR2 U80 ( .A(n93), .B(n105), .Z(n104) );
  GTECH_NOT U81 ( .A(A[7]), .Z(n105) );
  GTECH_NOT U82 ( .A(n99), .Z(n93) );
  GTECH_NOR2 U83 ( .A(n106), .B(n107), .Z(n99) );
  GTECH_NOT U84 ( .A(n4), .Z(n6) );
  GTECH_NOR2 U85 ( .A(n108), .B(n109), .Z(n4) );
  GTECH_NOR2 U86 ( .A(n110), .B(n111), .Z(n109) );
  GTECH_NOT U87 ( .A(n112), .Z(n111) );
  GTECH_NOR2 U88 ( .A(n112), .B(n113), .Z(n108) );
  GTECH_NOT U89 ( .A(n110), .Z(n113) );
  GTECH_NOR2 U90 ( .A(n114), .B(n115), .Z(n110) );
  GTECH_NOR2 U91 ( .A(n116), .B(n117), .Z(n112) );
  GTECH_NOR2 U92 ( .A(n118), .B(n119), .Z(n116) );
  GTECH_NOR2 U93 ( .A(B[7]), .B(n120), .Z(n119) );
  GTECH_NOT U94 ( .A(n121), .Z(n120) );
  GTECH_NOR2 U95 ( .A(n121), .B(n122), .Z(n118) );
  GTECH_NOT U96 ( .A(B[7]), .Z(n122) );
  GTECH_NOR2 U97 ( .A(n123), .B(n115), .Z(n121) );
  GTECH_NOT U98 ( .A(n124), .Z(P[6]) );
  GTECH_NOR2 U99 ( .A(n125), .B(n126), .Z(n124) );
  GTECH_NOR2 U100 ( .A(n127), .B(n123), .Z(n126) );
  GTECH_NOR2 U101 ( .A(n128), .B(n129), .Z(n125) );
  GTECH_NOT U102 ( .A(n127), .Z(n129) );
  GTECH_NOR2 U103 ( .A(n117), .B(n115), .Z(n127) );
  GTECH_NOT U104 ( .A(B[6]), .Z(n115) );
  GTECH_NOT U105 ( .A(n123), .Z(n128) );
  GTECH_NOR2 U106 ( .A(n130), .B(n131), .Z(n123) );
  GTECH_NOR2 U107 ( .A(n132), .B(n63), .Z(n131) );
  GTECH_NOT U108 ( .A(n133), .Z(n63) );
  GTECH_NOR2 U109 ( .A(n133), .B(n64), .Z(n130) );
  GTECH_NOT U110 ( .A(n132), .Z(n64) );
  GTECH_NOR2 U111 ( .A(n54), .B(n134), .Z(n132) );
  GTECH_NOT U112 ( .A(n135), .Z(n134) );
  GTECH_NOR2 U113 ( .A(n136), .B(n117), .Z(n135) );
  GTECH_NOR2 U114 ( .A(n137), .B(n62), .Z(n133) );
  GTECH_NOR2 U115 ( .A(n138), .B(n139), .Z(n62) );
  GTECH_NOT U116 ( .A(n140), .Z(n139) );
  GTECH_NOT U117 ( .A(n141), .Z(n138) );
  GTECH_NOR2 U118 ( .A(n140), .B(n141), .Z(n137) );
  GTECH_NOR2 U119 ( .A(n114), .B(n54), .Z(n141) );
  GTECH_NOR2 U120 ( .A(n142), .B(n51), .Z(n140) );
  GTECH_NOR2 U121 ( .A(n143), .B(n144), .Z(n51) );
  GTECH_NOT U122 ( .A(n145), .Z(n144) );
  GTECH_NOR2 U123 ( .A(n145), .B(n146), .Z(n142) );
  GTECH_NOT U124 ( .A(n143), .Z(n146) );
  GTECH_NOR2 U125 ( .A(n147), .B(n148), .Z(n143) );
  GTECH_NOR2 U126 ( .A(n149), .B(n52), .Z(n145) );
  GTECH_NOR2 U127 ( .A(n150), .B(n151), .Z(n52) );
  GTECH_NOT U128 ( .A(n152), .Z(n151) );
  GTECH_NOT U129 ( .A(n153), .Z(n150) );
  GTECH_NOR2 U130 ( .A(n152), .B(n153), .Z(n149) );
  GTECH_NOR2 U131 ( .A(n53), .B(n38), .Z(n153) );
  GTECH_NOR2 U132 ( .A(n154), .B(n35), .Z(n152) );
  GTECH_NOR2 U133 ( .A(n155), .B(n156), .Z(n35) );
  GTECH_NOT U134 ( .A(n157), .Z(n156) );
  GTECH_NOR2 U135 ( .A(n157), .B(n158), .Z(n154) );
  GTECH_NOT U136 ( .A(n155), .Z(n158) );
  GTECH_NOR2 U137 ( .A(n159), .B(n160), .Z(n155) );
  GTECH_NOR2 U138 ( .A(n161), .B(n36), .Z(n157) );
  GTECH_NOR2 U139 ( .A(n162), .B(n163), .Z(n36) );
  GTECH_NOT U140 ( .A(n164), .Z(n163) );
  GTECH_NOT U141 ( .A(n165), .Z(n162) );
  GTECH_NOR2 U142 ( .A(n164), .B(n165), .Z(n161) );
  GTECH_NOR2 U143 ( .A(n37), .B(n28), .Z(n165) );
  GTECH_NOR2 U144 ( .A(n166), .B(n25), .Z(n164) );
  GTECH_NOR2 U145 ( .A(n167), .B(n168), .Z(n25) );
  GTECH_NOT U146 ( .A(n169), .Z(n168) );
  GTECH_NOR2 U147 ( .A(n169), .B(n170), .Z(n166) );
  GTECH_NOT U148 ( .A(n167), .Z(n170) );
  GTECH_NOR2 U149 ( .A(n171), .B(n172), .Z(n167) );
  GTECH_NOR2 U150 ( .A(n173), .B(n26), .Z(n169) );
  GTECH_NOR2 U151 ( .A(n174), .B(n175), .Z(n26) );
  GTECH_NOT U152 ( .A(n176), .Z(n175) );
  GTECH_NOT U153 ( .A(n177), .Z(n174) );
  GTECH_NOR2 U154 ( .A(n176), .B(n177), .Z(n173) );
  GTECH_NOR2 U155 ( .A(n27), .B(n90), .Z(n177) );
  GTECH_NOR2 U156 ( .A(n178), .B(n79), .Z(n176) );
  GTECH_NOR2 U157 ( .A(n179), .B(n180), .Z(n79) );
  GTECH_NOT U158 ( .A(n181), .Z(n180) );
  GTECH_NOR2 U159 ( .A(n181), .B(n182), .Z(n178) );
  GTECH_NOT U160 ( .A(n179), .Z(n182) );
  GTECH_NOR2 U161 ( .A(n183), .B(n184), .Z(n179) );
  GTECH_NOR2 U162 ( .A(n185), .B(n186), .Z(n181) );
  GTECH_NOT U163 ( .A(n187), .Z(n186) );
  GTECH_NOR2 U164 ( .A(n188), .B(n189), .Z(n187) );
  GTECH_NOR2 U165 ( .A(B[0]), .B(A[5]), .Z(n189) );
  GTECH_NOR2 U166 ( .A(A[6]), .B(n190), .Z(n188) );
  GTECH_NOT U167 ( .A(n191), .Z(n185) );
  GTECH_NOR2 U168 ( .A(n97), .B(n81), .Z(n191) );
  GTECH_NOR2 U169 ( .A(n107), .B(n192), .Z(n81) );
  GTECH_NOT U170 ( .A(n193), .Z(n192) );
  GTECH_NOR2 U171 ( .A(n92), .B(n194), .Z(n193) );
  GTECH_NOT U172 ( .A(A[6]), .Z(n107) );
  GTECH_NOT U173 ( .A(n195), .Z(P[5]) );
  GTECH_NOR2 U174 ( .A(n196), .B(n197), .Z(n195) );
  GTECH_NOR2 U175 ( .A(n198), .B(n136), .Z(n197) );
  GTECH_NOR2 U176 ( .A(n199), .B(n200), .Z(n136) );
  GTECH_NOR2 U177 ( .A(n201), .B(n202), .Z(n200) );
  GTECH_NOR2 U178 ( .A(n203), .B(n204), .Z(n199) );
  GTECH_NOR2 U179 ( .A(n205), .B(n206), .Z(n196) );
  GTECH_NOT U180 ( .A(n198), .Z(n206) );
  GTECH_NOR2 U181 ( .A(n117), .B(n54), .Z(n198) );
  GTECH_NOT U182 ( .A(B[5]), .Z(n54) );
  GTECH_NOR2 U183 ( .A(n207), .B(n148), .Z(n205) );
  GTECH_NOR2 U184 ( .A(n202), .B(n204), .Z(n148) );
  GTECH_NOT U185 ( .A(n201), .Z(n204) );
  GTECH_NOT U186 ( .A(n203), .Z(n202) );
  GTECH_NOR2 U187 ( .A(n201), .B(n203), .Z(n207) );
  GTECH_NOR2 U188 ( .A(n208), .B(n147), .Z(n203) );
  GTECH_NOR2 U189 ( .A(n209), .B(n210), .Z(n147) );
  GTECH_NOT U190 ( .A(n211), .Z(n210) );
  GTECH_NOT U191 ( .A(n212), .Z(n209) );
  GTECH_NOR2 U192 ( .A(n211), .B(n212), .Z(n208) );
  GTECH_NOR2 U193 ( .A(n114), .B(n38), .Z(n212) );
  GTECH_NOR2 U194 ( .A(n213), .B(n160), .Z(n211) );
  GTECH_NOR2 U195 ( .A(n214), .B(n215), .Z(n160) );
  GTECH_NOT U196 ( .A(n216), .Z(n215) );
  GTECH_NOR2 U197 ( .A(n216), .B(n217), .Z(n213) );
  GTECH_NOT U198 ( .A(n214), .Z(n217) );
  GTECH_NOR2 U199 ( .A(n218), .B(n219), .Z(n214) );
  GTECH_NOR2 U200 ( .A(n220), .B(n159), .Z(n216) );
  GTECH_NOR2 U201 ( .A(n221), .B(n222), .Z(n159) );
  GTECH_NOT U202 ( .A(n223), .Z(n222) );
  GTECH_NOT U203 ( .A(n224), .Z(n221) );
  GTECH_NOR2 U204 ( .A(n223), .B(n224), .Z(n220) );
  GTECH_NOR2 U205 ( .A(n53), .B(n28), .Z(n224) );
  GTECH_NOR2 U206 ( .A(n225), .B(n172), .Z(n223) );
  GTECH_NOR2 U207 ( .A(n226), .B(n227), .Z(n172) );
  GTECH_NOT U208 ( .A(n228), .Z(n227) );
  GTECH_NOR2 U209 ( .A(n228), .B(n229), .Z(n225) );
  GTECH_NOT U210 ( .A(n226), .Z(n229) );
  GTECH_NOR2 U211 ( .A(n230), .B(n231), .Z(n226) );
  GTECH_NOR2 U212 ( .A(n232), .B(n171), .Z(n228) );
  GTECH_NOR2 U213 ( .A(n233), .B(n234), .Z(n171) );
  GTECH_NOT U214 ( .A(n235), .Z(n234) );
  GTECH_NOT U215 ( .A(n236), .Z(n233) );
  GTECH_NOR2 U216 ( .A(n235), .B(n236), .Z(n232) );
  GTECH_NOR2 U217 ( .A(n37), .B(n90), .Z(n236) );
  GTECH_NOR2 U218 ( .A(n237), .B(n184), .Z(n235) );
  GTECH_NOR2 U219 ( .A(n238), .B(n239), .Z(n184) );
  GTECH_NOT U220 ( .A(n240), .Z(n239) );
  GTECH_NOR2 U221 ( .A(n241), .B(n242), .Z(n238) );
  GTECH_NOR2 U222 ( .A(n242), .B(n243), .Z(n237) );
  GTECH_NOT U223 ( .A(n244), .Z(n243) );
  GTECH_NOR2 U224 ( .A(n241), .B(n240), .Z(n244) );
  GTECH_NOR2 U225 ( .A(n245), .B(n246), .Z(n240) );
  GTECH_NOT U226 ( .A(n247), .Z(n246) );
  GTECH_NOR2 U227 ( .A(n248), .B(n249), .Z(n247) );
  GTECH_NOR2 U228 ( .A(B[0]), .B(A[4]), .Z(n249) );
  GTECH_NOR2 U229 ( .A(A[5]), .B(n250), .Z(n248) );
  GTECH_NOR2 U230 ( .A(n27), .B(n106), .Z(n250) );
  GTECH_NOT U231 ( .A(n251), .Z(n245) );
  GTECH_NOR2 U232 ( .A(n97), .B(n183), .Z(n251) );
  GTECH_NOR2 U233 ( .A(n252), .B(n194), .Z(n183) );
  GTECH_NOT U234 ( .A(n190), .Z(n194) );
  GTECH_NOR2 U235 ( .A(n106), .B(n89), .Z(n190) );
  GTECH_NOT U236 ( .A(A[5]), .Z(n89) );
  GTECH_NOR2 U237 ( .A(n38), .B(n253), .Z(n201) );
  GTECH_NOT U238 ( .A(n254), .Z(n253) );
  GTECH_NOR2 U239 ( .A(n255), .B(n117), .Z(n254) );
  GTECH_NOT U240 ( .A(n256), .Z(P[4]) );
  GTECH_NOR2 U241 ( .A(n257), .B(n258), .Z(n256) );
  GTECH_NOR2 U242 ( .A(n259), .B(n255), .Z(n258) );
  GTECH_NOR2 U243 ( .A(n260), .B(n261), .Z(n255) );
  GTECH_NOR2 U244 ( .A(n262), .B(n263), .Z(n261) );
  GTECH_NOR2 U245 ( .A(n264), .B(n265), .Z(n260) );
  GTECH_NOR2 U246 ( .A(n266), .B(n267), .Z(n257) );
  GTECH_NOT U247 ( .A(n259), .Z(n267) );
  GTECH_NOR2 U248 ( .A(n117), .B(n38), .Z(n259) );
  GTECH_NOT U249 ( .A(B[4]), .Z(n38) );
  GTECH_NOR2 U250 ( .A(n268), .B(n219), .Z(n266) );
  GTECH_NOR2 U251 ( .A(n263), .B(n265), .Z(n219) );
  GTECH_NOT U252 ( .A(n262), .Z(n265) );
  GTECH_NOT U253 ( .A(n264), .Z(n263) );
  GTECH_NOR2 U254 ( .A(n262), .B(n264), .Z(n268) );
  GTECH_NOR2 U255 ( .A(n269), .B(n218), .Z(n264) );
  GTECH_NOR2 U256 ( .A(n270), .B(n271), .Z(n218) );
  GTECH_NOT U257 ( .A(n272), .Z(n271) );
  GTECH_NOT U258 ( .A(n273), .Z(n270) );
  GTECH_NOR2 U259 ( .A(n272), .B(n273), .Z(n269) );
  GTECH_NOR2 U260 ( .A(n114), .B(n28), .Z(n273) );
  GTECH_NOR2 U261 ( .A(n274), .B(n231), .Z(n272) );
  GTECH_NOR2 U262 ( .A(n275), .B(n276), .Z(n231) );
  GTECH_NOT U263 ( .A(n277), .Z(n276) );
  GTECH_NOR2 U264 ( .A(n277), .B(n278), .Z(n274) );
  GTECH_NOT U265 ( .A(n275), .Z(n278) );
  GTECH_NOR2 U266 ( .A(n279), .B(n280), .Z(n275) );
  GTECH_NOR2 U267 ( .A(n281), .B(n230), .Z(n277) );
  GTECH_NOR2 U268 ( .A(n282), .B(n283), .Z(n230) );
  GTECH_NOT U269 ( .A(n284), .Z(n283) );
  GTECH_NOT U270 ( .A(n285), .Z(n282) );
  GTECH_NOR2 U271 ( .A(n284), .B(n285), .Z(n281) );
  GTECH_NOR2 U272 ( .A(n53), .B(n90), .Z(n285) );
  GTECH_NOR2 U273 ( .A(n286), .B(n242), .Z(n284) );
  GTECH_NOR2 U274 ( .A(n287), .B(n288), .Z(n242) );
  GTECH_NOT U275 ( .A(n289), .Z(n288) );
  GTECH_NOR2 U276 ( .A(n289), .B(n290), .Z(n286) );
  GTECH_NOT U277 ( .A(n287), .Z(n290) );
  GTECH_NOR2 U278 ( .A(n291), .B(n292), .Z(n287) );
  GTECH_NOR2 U279 ( .A(n293), .B(n294), .Z(n289) );
  GTECH_NOT U280 ( .A(n295), .Z(n294) );
  GTECH_NOR2 U281 ( .A(n296), .B(n297), .Z(n295) );
  GTECH_NOR2 U282 ( .A(B[1]), .B(A[4]), .Z(n297) );
  GTECH_NOR2 U283 ( .A(A[3]), .B(n298), .Z(n296) );
  GTECH_NOT U284 ( .A(n299), .Z(n293) );
  GTECH_NOR2 U285 ( .A(n241), .B(n97), .Z(n299) );
  GTECH_NOR2 U286 ( .A(n106), .B(n300), .Z(n241) );
  GTECH_NOT U287 ( .A(n301), .Z(n300) );
  GTECH_NOR2 U288 ( .A(n252), .B(n37), .Z(n301) );
  GTECH_NOT U289 ( .A(n298), .Z(n252) );
  GTECH_NOR2 U290 ( .A(n27), .B(n92), .Z(n298) );
  GTECH_NOT U291 ( .A(A[4]), .Z(n27) );
  GTECH_NOR2 U292 ( .A(n28), .B(n302), .Z(n262) );
  GTECH_NOT U293 ( .A(n303), .Z(n302) );
  GTECH_NOR2 U294 ( .A(n304), .B(n117), .Z(n303) );
  GTECH_NOT U295 ( .A(n305), .Z(P[3]) );
  GTECH_NOR2 U296 ( .A(n306), .B(n307), .Z(n305) );
  GTECH_NOR2 U297 ( .A(n308), .B(n304), .Z(n307) );
  GTECH_NOR2 U298 ( .A(n309), .B(n310), .Z(n304) );
  GTECH_NOR2 U299 ( .A(n311), .B(n312), .Z(n310) );
  GTECH_NOR2 U300 ( .A(n313), .B(n314), .Z(n309) );
  GTECH_NOR2 U301 ( .A(n315), .B(n316), .Z(n306) );
  GTECH_NOT U302 ( .A(n308), .Z(n316) );
  GTECH_NOR2 U303 ( .A(n117), .B(n28), .Z(n308) );
  GTECH_NOT U304 ( .A(B[3]), .Z(n28) );
  GTECH_NOR2 U305 ( .A(n317), .B(n280), .Z(n315) );
  GTECH_NOR2 U306 ( .A(n312), .B(n314), .Z(n280) );
  GTECH_NOT U307 ( .A(n311), .Z(n314) );
  GTECH_NOT U308 ( .A(n313), .Z(n312) );
  GTECH_NOR2 U309 ( .A(n311), .B(n313), .Z(n317) );
  GTECH_NOR2 U310 ( .A(n318), .B(n279), .Z(n313) );
  GTECH_NOR2 U311 ( .A(n319), .B(n320), .Z(n279) );
  GTECH_NOT U312 ( .A(n321), .Z(n320) );
  GTECH_NOT U313 ( .A(n322), .Z(n319) );
  GTECH_NOR2 U314 ( .A(n321), .B(n322), .Z(n318) );
  GTECH_NOR2 U315 ( .A(n114), .B(n90), .Z(n322) );
  GTECH_NOR2 U316 ( .A(n323), .B(n292), .Z(n321) );
  GTECH_NOR2 U317 ( .A(n324), .B(n325), .Z(n292) );
  GTECH_NOT U318 ( .A(n326), .Z(n325) );
  GTECH_NOR2 U319 ( .A(n326), .B(n327), .Z(n323) );
  GTECH_NOT U320 ( .A(n324), .Z(n327) );
  GTECH_NOR2 U321 ( .A(n328), .B(n329), .Z(n324) );
  GTECH_NOR2 U322 ( .A(n330), .B(n331), .Z(n326) );
  GTECH_NOT U323 ( .A(n332), .Z(n331) );
  GTECH_NOR2 U324 ( .A(n333), .B(n334), .Z(n332) );
  GTECH_NOR2 U325 ( .A(B[0]), .B(A[2]), .Z(n334) );
  GTECH_NOR2 U326 ( .A(A[3]), .B(n335), .Z(n333) );
  GTECH_NOR2 U327 ( .A(n106), .B(n53), .Z(n335) );
  GTECH_NOT U328 ( .A(n336), .Z(n330) );
  GTECH_NOR2 U329 ( .A(n97), .B(n291), .Z(n336) );
  GTECH_NOR2 U330 ( .A(n337), .B(n338), .Z(n291) );
  GTECH_NOT U331 ( .A(n339), .Z(n338) );
  GTECH_NOR2 U332 ( .A(n37), .B(n106), .Z(n339) );
  GTECH_NOT U333 ( .A(A[3]), .Z(n37) );
  GTECH_NOR2 U334 ( .A(n90), .B(n340), .Z(n311) );
  GTECH_NOT U335 ( .A(n341), .Z(n340) );
  GTECH_NOR2 U336 ( .A(n342), .B(n117), .Z(n341) );
  GTECH_NOT U337 ( .A(n343), .Z(P[2]) );
  GTECH_NOR2 U338 ( .A(n344), .B(n345), .Z(n343) );
  GTECH_NOR2 U339 ( .A(n346), .B(n342), .Z(n345) );
  GTECH_NOR2 U340 ( .A(n347), .B(n348), .Z(n344) );
  GTECH_NOT U341 ( .A(n346), .Z(n348) );
  GTECH_NOR2 U342 ( .A(n117), .B(n90), .Z(n346) );
  GTECH_NOT U343 ( .A(B[2]), .Z(n90) );
  GTECH_NOT U344 ( .A(n342), .Z(n347) );
  GTECH_NOR2 U345 ( .A(n349), .B(n350), .Z(n342) );
  GTECH_NOR2 U346 ( .A(n328), .B(n351), .Z(n350) );
  GTECH_NOT U347 ( .A(n352), .Z(n351) );
  GTECH_NOR2 U348 ( .A(n352), .B(n353), .Z(n349) );
  GTECH_NOT U349 ( .A(n328), .Z(n353) );
  GTECH_NOR2 U350 ( .A(n114), .B(n354), .Z(n328) );
  GTECH_NOT U351 ( .A(n355), .Z(n354) );
  GTECH_NOR2 U352 ( .A(n106), .B(n356), .Z(n355) );
  GTECH_NOT U353 ( .A(P[0]), .Z(n356) );
  GTECH_NOR2 U354 ( .A(n357), .B(n358), .Z(n352) );
  GTECH_NOT U355 ( .A(n359), .Z(n358) );
  GTECH_NOR2 U356 ( .A(n360), .B(n361), .Z(n359) );
  GTECH_NOR2 U357 ( .A(B[1]), .B(A[2]), .Z(n361) );
  GTECH_NOR2 U358 ( .A(A[1]), .B(n362), .Z(n360) );
  GTECH_NOT U359 ( .A(n363), .Z(n357) );
  GTECH_NOR2 U360 ( .A(n329), .B(n97), .Z(n363) );
  GTECH_NOR2 U361 ( .A(B[1]), .B(B[0]), .Z(n97) );
  GTECH_NOR2 U362 ( .A(n114), .B(n364), .Z(n329) );
  GTECH_NOT U363 ( .A(n365), .Z(n364) );
  GTECH_NOR2 U364 ( .A(n106), .B(n337), .Z(n365) );
  GTECH_NOT U365 ( .A(n362), .Z(n337) );
  GTECH_NOR2 U366 ( .A(n92), .B(n53), .Z(n362) );
  GTECH_NOT U367 ( .A(A[2]), .Z(n53) );
  GTECH_NOT U368 ( .A(n366), .Z(P[1]) );
  GTECH_NOR2 U369 ( .A(n367), .B(n368), .Z(n366) );
  GTECH_NOR2 U370 ( .A(n369), .B(n370), .Z(n368) );
  GTECH_NOT U371 ( .A(n371), .Z(n370) );
  GTECH_NOR2 U372 ( .A(n371), .B(n372), .Z(n367) );
  GTECH_NOT U373 ( .A(n369), .Z(n372) );
  GTECH_NOR2 U374 ( .A(n92), .B(n114), .Z(n369) );
  GTECH_NOT U375 ( .A(A[1]), .Z(n114) );
  GTECH_NOR2 U376 ( .A(n106), .B(n117), .Z(n371) );
  GTECH_NOT U377 ( .A(B[1]), .Z(n106) );
  GTECH_NOR2 U378 ( .A(n92), .B(n117), .Z(P[0]) );
  GTECH_NOT U379 ( .A(A[0]), .Z(n117) );
  GTECH_NOT U380 ( .A(B[0]), .Z(n92) );
endmodule

