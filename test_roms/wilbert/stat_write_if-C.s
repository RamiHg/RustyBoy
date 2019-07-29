; This file is part of Mooneye GB.
; Copyright (C) 2014-2016 Joonas Javanainen <joonas.javanainen@gmail.com>
;
; Mooneye GB is free software: you can redistribute it and/or modify
; it under the terms of the GNU General Public License as published by
; the Free Software Foundation, either version 3 of the License, or
; (at your option) any later version.
;
; Mooneye GB is distributed in the hope that it will be useful,
; but WITHOUT ANY WARRANTY; without even the implied warranty of
; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
; GNU General Public License for more details.
;
; You should have received a copy of the GNU General Public License
; along with Mooneye GB.  If not, see <http://www.gnu.org/licenses/>.

; Tests effects of writing to STAT register during various modes.

; Verified results:
;   pass: CGB, AGS
;   fail: MGB
;   untested: DNG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

.macro clear_interrupts
  xor a
  ldh (<IF), a
.endm


  di
  wait_vblank
  disable_lcd
  call reset_screen
  call print_load_font
  ld a,$FF
  ld (testcase_id),a
  ld a,$f0
  ldh (<LYC),a
  enable_lcd

.macro testcase_0 ARGS stat1 stat2 if if_cgb
  ; wait for mode 0
  ld a,(testcase_id)
  inc a
  ld (testcase_id), a
  ld d,stat1
  ld e,stat2
  ld h,if_cgb
  call run_testcase_0
.endm

.macro testcase_1 ARGS stat1 stat2 if if_cgb
  ; wait for mode 1
  ld a,(testcase_id)
  inc a
  ld (testcase_id), a
  ld d,stat1
  ld e,stat2
  ld h,if_cgb
  call run_testcase_1
.endm

.macro testcase_2 ARGS stat1 stat2 if if_cgb
  ; wait for mode 2
  ld a,(testcase_id)
  inc a
  ld (testcase_id), a
  ld d,stat1
  ld e,stat2
  ld h,if_cgb
  call run_testcase_2
.endm

.macro testcase_3 ARGS stat1 stat2 if if_cgb
  ; wait for mode 3
  ld a,(testcase_id)
  inc a
  ld (testcase_id), a
  ld d,stat1
  ld e,stat2
  ld h,if_cgb
  call run_testcase_3
.endm

.macro testcase_line ARGS stat1 stat2 if if_cgb
  ; wait for mode 3 and line
  ld a,(testcase_id)
  inc a
  ld (testcase_id), a
  ld d,stat1
  ld e,stat2
  ld h,if
  call run_testcase_line
.endm

  ; test mode to wait for
  ; start STAT value
  ; STAT value to write during test mode
  ; expected to see STAT irq on dmg/mgb/sgb/sgb2
  ; expected to see STAT irq on cgb/agb/ags
  ; nothing set
  testcase_0 $00, $00, 2, 0
  testcase_0 $00, $08, 2, 2
  testcase_0 $00, $10, 2, 0
  testcase_0 $00, $20, 2, 0
  testcase_0 $00, $40, 2, 0
  testcase_1 $00, $00, 2, 0
  testcase_1 $00, $08, 2, 0
  testcase_1 $00, $10, 2, 2
  testcase_1 $00, $20, 2, 0
  testcase_1 $00, $40, 2, 0
  testcase_2 $00, $00, 0, 0
  testcase_2 $00, $08, 0, 0
  testcase_2 $00, $10, 0, 0
  testcase_2 $00, $20, 0, 0
  testcase_2 $00, $40, 0, 0
  testcase_3 $00, $00, 0, 0
  testcase_3 $00, $08, 0, 0
  testcase_3 $00, $10, 0, 0
  testcase_3 $00, $20, 0, 0
  testcase_3 $00, $40, 0, 0
  ; $08 set
  ; #14
  testcase_0 $08, $00, 0, 0
  testcase_0 $08, $08, 0, 0
  testcase_0 $08, $10, 0, 0
  testcase_0 $08, $20, 0, 0
  testcase_0 $08, $40, 0, 0
  testcase_1 $08, $00, 2, 0
  testcase_1 $08, $08, 2, 0
  testcase_1 $08, $10, 2, 2
  testcase_1 $08, $20, 2, 0
  testcase_1 $08, $40, 2, 0
  testcase_2 $08, $00, 0, 0
  testcase_2 $08, $08, 0, 0
  testcase_2 $08, $10, 0, 0
  testcase_2 $08, $20, 0, 0
  testcase_2 $08, $40, 0, 0
  testcase_3 $08, $00, 0, 0
  testcase_3 $08, $08, 0, 0
  testcase_3 $08, $10, 0, 0
  testcase_3 $08, $20, 0, 0
  testcase_3 $08, $40, 0, 0
  ; $10 set
  ; #28
  testcase_0 $10, $00, 2, 0
  testcase_0 $10, $08, 2, 2
  testcase_0 $10, $10, 2, 0
  testcase_0 $10, $20, 2, 0
  testcase_0 $10, $40, 2, 0
  testcase_1 $10, $00, 0, 0
  testcase_1 $10, $08, 0, 0
  testcase_1 $10, $10, 0, 0
  testcase_1 $10, $20, 0, 0
  testcase_1 $10, $40, 0, 0
  testcase_2 $10, $00, 0, 0
  testcase_2 $10, $08, 0, 0
  testcase_2 $10, $10, 0, 0
  testcase_2 $10, $20, 0, 0
  testcase_2 $10, $40, 0, 0
  testcase_3 $10, $00, 0, 0
  testcase_3 $10, $08, 0, 0
  testcase_3 $10, $10, 0, 0
  testcase_3 $10, $20, 0, 0
  testcase_3 $10, $40, 0, 0
  ; $20 set
  ; #3C
  testcase_0 $20, $00, 2, 0
  testcase_0 $20, $08, 2, 2
  testcase_0 $20, $10, 2, 0
  testcase_0 $20, $20, 2, 0
  testcase_0 $20, $40, 2, 0
  testcase_1 $20, $00, 2, 0
  testcase_1 $20, $08, 2, 0
  testcase_1 $20, $10, 2, 2
  testcase_1 $20, $20, 2, 0
  testcase_1 $20, $40, 2, 0
  testcase_2 $20, $00, 0, 0
  testcase_2 $20, $08, 0, 0
  testcase_2 $20, $10, 0, 0
  testcase_2 $20, $20, 0, 0
  testcase_2 $20, $40, 0, 0
  testcase_3 $20, $00, 0, 0
  testcase_3 $20, $08, 0, 0
  testcase_3 $20, $10, 0, 0
  testcase_3 $20, $20, 0, 0
  testcase_3 $20, $40, 0, 0
  ; other cases
  testcase_0 $00, $40, 2, 0
  testcase_1 $08, $10, 2, 2
  testcase_line $40, $00, 0, 0
  testcase_line $40, $40, 0, 0
  testcase_line $00, $40, 2, 2
  
  test_ok

.macro wait_not_mode ARGS mode
- ldh a, (<STAT)
  and $03
  cp mode
  jr z, -
.endm

.macro wait_mode ARGS mode
- ldh a, (<STAT)
  and $03
  cp mode
  jr nz, -
.endm

run_testcase_0:
  ; wait for mode 0
  push hl
  wait_ly 144
  disable_lcd
  ld a,d
  ldh (<STAT),a
  enable_lcd
  pop hl
  wait_ly 10
  wait_not_mode 0
  wait_mode 0
  clear_interrupts
  ld a,e
  ldh (<STAT),a
  ldh a,(<IF)
  and $02
  cp h
  ret z
  jp test_fail

run_testcase_1:
  ; wait for mode 1
  push hl
  wait_ly 144
  disable_lcd
  ld a,d
  ldh (<STAT),a
  enable_lcd
  pop hl
  wait_ly 142
  wait_not_mode 1
  wait_mode 1
  clear_interrupts
  ld a,e
  ldh (<STAT),a
  ldh a,(<IF)
  and $02
  cp h
  ret z
  jp test_fail

run_testcase_2:
  ; wait for mode 2
  push hl
  wait_ly 144
  disable_lcd
  ld a,d
  ldh (<STAT),a
  enable_lcd
  pop hl
  wait_ly 10
  wait_not_mode 2
  wait_mode 2
  clear_interrupts
  ld a,e
  ldh (<STAT),a
  ldh a,(<IF)
  and $02
  cp h
  ret z
  jp test_fail

run_testcase_3:
  ; wait for mode 3
  push hl
  wait_ly 144
  disable_lcd
  ld a,d
  ldh (<STAT),a
  enable_lcd
  pop hl
  wait_ly 10
  wait_not_mode 3
  wait_mode 3
  clear_interrupts
  ld a,e
  ldh (<STAT),a
  ldh a,(<IF)
  and $02
  cp h
  ret z
  jp test_fail

run_testcase_line:
  ; wait for mode 3
  push hl
  wait_ly 144
  disable_lcd
  ld a,d
  ldh (<STAT),a
  enable_lcd
  pop hl
  ld a,10
  ldh (<LYC),a
  wait_ly 10
  wait_not_mode 3
  wait_mode 3
  clear_interrupts
  ld a,e
  ldh (<STAT),a
  ldh a,(<IF)
  push af
  ld a,$f0
  ldh (<LYC),a
  pop af
  and $02
  cp h
  ret z
  jp test_fail

test_fail_dump:
  save_results
  jp process_results

test_fail:
  print_results _test_fail_cb
_test_fail_cb:
  print_string_literal "TEST #"
  ld a, (testcase_id)
  call print_a
  print_string_literal " FAILED"
  ld d, $42
  ret

.ramsection "Test-State" slot 2
  testcase_id dw
.ends
