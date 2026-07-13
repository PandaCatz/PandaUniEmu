"""Generate project-owned NROM images and pinned py65 architectural traces."""

from __future__ import annotations

import argparse
import hashlib
import sys
import types
from pathlib import Path

PY65_COMMIT = "3138e1b337734a9b2ac1ea90ee7a453514436221"
PY65_CPU_SHA256 = "bdae2b7ef3e2a38519a007412280107f330c6fc6433738364578fe8338e57e7e"
PY65_LICENSE_SHA256 = "aff1cd260d7d6367ccc9ecb28e6823d54ec7cfd254c27e43ae76c2747a7dc6a1"
PY65_IMPORT_HASHES = {
    "py65/__init__.py": "9c3218570ae3ad9bc4ca97809f9f24f490ac94b4d2557970b8ebb57dbbb87c7e",
    "py65/devices/__init__.py": "9c3218570ae3ad9bc4ca97809f9f24f490ac94b4d2557970b8ebb57dbbb87c7e",
    "py65/devices/mpu6502.py": PY65_CPU_SHA256,
    "py65/utils/__init__.py": "9c3218570ae3ad9bc4ca97809f9f24f490ac94b4d2557970b8ebb57dbbb87c7e",
    "py65/utils/conversions.py": "8c1a947f24351ced8265dae7e94eb8b13b24f489a4f62f15040af1a597997d44",
    "py65/utils/devices.py": "1ab16cbe2c6ae2213452d9ff577dec3856df209e70361e1f1598ffb7cbbf0a1c",
}
START = 0xC000
SENTINEL = 0xC102
MAX_STEPS = 64
TRAINER_LEN = 512


def trainer_byte(index: int) -> int:
    if not 0 <= index < TRAINER_LEN:
        raise ValueError("trainer index is out of range")
    return (index * 37 + 0xA7) & 0xFF


TRAINER = bytes(trainer_byte(index) for index in range(TRAINER_LEN))


def build_program() -> bytes:
    program = bytearray(SENTINEL - START + 1)
    occupied = bytearray(len(program))

    def place(address: int, data: bytes) -> None:
        offset = address - START
        end = offset + len(data)
        if offset < 0 or end > len(program):
            raise ValueError(f"program placement ${address:04X} is out of range")
        if any(occupied[offset:end]):
            raise ValueError(f"program placement ${address:04X} overlaps existing bytes")
        program[offset:end] = data
        occupied[offset:end] = bytes([1]) * len(data)

    place(
        0xC000,
        bytes(
            [
                0xD8,  # CLD
                0xA2,
                0xFD,  # LDX #$FD
                0x9A,  # TXS
                0xA9,
                0x12,  # LDA #$12
                0x8D,
                0x02,
                0x00,  # STA $0002
                0xAD,
                0x02,
                0x08,  # LDA $0802 (RAM mirror)
                0x18,  # CLC
                0x69,
                0x34,  # ADC #$34, with decimal disabled
                0x8D,
                0xFF,
                0x17,  # STA $17FF (RAM mirror)
                0xAD,
                0xFF,
                0x07,  # LDA $07FF
                0x48,  # PHA
                0xA9,
                0x00,  # LDA #$00
                0x68,  # PLA
                0x20,
                0x80,
                0xC0,  # JSR $C080
                0x8D,
                0x00,
                0x60,  # STA $6000 (PRG RAM)
                0xAD,
                0x00,
                0x60,  # LDA $6000 (prove PRG RAM retained the write)
                0xA9,
                0xA5,  # LDA #$A5
                0x8D,
                0x00,
                0x80,  # STA $8000 (must not mutate PRG ROM)
                0xAD,
                0x00,
                0x80,  # LDA $8000 (NROM-128 mirror / NROM-256 low bank)
                0x8D,
                0x01,
                0x60,  # STA $6001
                0xAD,
                0x01,
                0x60,  # LDA $6001 (prove the mapper-specific value persisted)
                0xAD,
                0x00,
                0x70,  # LDA $7000 (trainer start / ordinary PRG RAM)
                0x8D,
                0x02,
                0x60,  # STA $6002
                0xAD,
                0x02,
                0x60,  # LDA $6002 (prove the start value persisted)
                0xAD,
                0xFF,
                0x71,  # LDA $71FF (trainer end / ordinary PRG RAM)
                0x8D,
                0x03,
                0x60,  # STA $6003
                0xAD,
                0x03,
                0x60,  # LDA $6003 (prove the end value persisted)
                0xA0,
                0x03,  # LDY #$03
                0x88,  # loop: DEY
                0xD0,
                0xFD,  # BNE loop
                0xA9,
                0x80,  # LDA #$80
                0x8D,
                0x00,
                0x01,  # STA $0100
                0xA2,
                0x01,  # LDX #$01
                0xBD,
                0xFF,
                0x00,  # LDA $00FF,X (page-cross cycle)
                0x4C,
                0xFB,
                0xC0,  # JMP $C0FB
            ]
        ),
    )
    place(0xC080, bytes([0x49, 0xFF, 0x38, 0x2A, 0x60]))  # EOR, SEC, ROL A, RTS
    place(0xC0FB, bytes([0xA9, 0x00, 0xF0, 0x01, 0xEA]))  # crossing BEQ
    place(0xC100, bytes([0xA9, 0x5A, 0xEA]))  # final NOP is the sentinel
    return bytes(program)


PROGRAM = build_program()


def build_image(prg_banks: int, trainer: bool = False) -> bytes:
    if prg_banks not in (1, 2):
        raise ValueError("clean-room image supports only one or two PRG banks")
    prg = bytearray(prg_banks * 16 * 1024)
    program_offset = 0 if prg_banks == 1 else 16 * 1024
    prg[program_offset : program_offset + len(PROGRAM)] = PROGRAM
    if prg_banks == 2:
        prg[0] = 0x5A
    prg[-4:-2] = bytes([START & 0xFF, START >> 8])

    header = bytearray(16)
    header[:4] = b"NES\x1a"
    header[4] = prg_banks
    header[5] = 1
    header[6] = int(trainer) << 2
    header[8] = 1
    trainer_bytes = TRAINER if trainer else b""
    return bytes(header + trainer_bytes + prg + bytearray(8 * 1024))


class NromMemory:
    def __init__(self, image: bytes):
        if len(image) < 16 or image[:4] != b"NES\x1a":
            raise ValueError("oracle image has an invalid iNES header")
        prg_banks = image[4]
        if prg_banks not in (1, 2):
            raise ValueError("oracle image is not NROM-128 or NROM-256")
        if image[5] != 1 or image[6] not in (0, 0x04) or image[7] != 0 or image[8] != 1:
            raise ValueError("oracle image has an unexpected iNES configuration")
        if any(image[9:16]):
            raise ValueError("oracle image has nonzero reserved header bytes")
        trainer_len = TRAINER_LEN if image[6] & 0x04 else 0
        expected_length = 16 + trainer_len + prg_banks * 16 * 1024 + 8 * 1024
        if len(image) != expected_length:
            raise ValueError("oracle image has a truncated or trailing payload")
        prg_start = 16 + trainer_len
        self.prg_rom = bytes(image[prg_start : prg_start + prg_banks * 16 * 1024])
        self.ram = bytearray(2 * 1024)
        self.prg_ram = bytearray(8 * 1024)
        if trainer_len:
            self.prg_ram[0x1000:0x1200] = image[16 : 16 + TRAINER_LEN]

    def __getitem__(self, address: int) -> int:
        if not isinstance(address, int) or not 0 <= address <= 0xFFFF:
            raise IndexError("oracle address is outside the 16-bit bus")
        if address < 0x2000:
            return self.ram[address & 0x07FF]
        if 0x6000 <= address < 0x8000:
            return self.prg_ram[address - 0x6000]
        if address >= 0x8000:
            offset = address - 0x8000
            if len(self.prg_rom) == 16 * 1024:
                offset &= 0x3FFF
            return self.prg_rom[offset]
        raise RuntimeError(f"oracle touched unsupported address ${address:04X}")

    def __setitem__(self, address: int, value: int) -> None:
        if not isinstance(address, int) or not 0 <= address <= 0xFFFF:
            raise IndexError("oracle address is outside the 16-bit bus")
        if not isinstance(value, int) or not 0 <= value <= 0xFF:
            raise ValueError("oracle write is outside the byte range")
        if address < 0x2000:
            self.ram[address & 0x07FF] = value
            return
        if 0x6000 <= address < 0x8000:
            self.prg_ram[address - 0x6000] = value
            return
        if address >= 0x8000:
            return
        raise RuntimeError(f"oracle touched unsupported address ${address:04X}")


LENGTHS = {
    "acc": 1,
    "imp": 1,
    "imm": 2,
    "rel": 2,
    "abs": 3,
    "abx": 3,
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require_pinned_py65(root: Path):
    required_hashes = {"LICENSE.txt": PY65_LICENSE_SHA256, **PY65_IMPORT_HASHES}
    validated_files: dict[str, bytes] = {}
    for relative_path, expected in required_hashes.items():
        path = root / relative_path
        if not path.is_file():
            raise RuntimeError(f"missing or oversized pinned py65 file: {path}")
        source = path.read_bytes()
        if len(source) > 1_000_000:
            raise RuntimeError(f"missing or oversized pinned py65 file: {path}")
        actual = sha256(source)
        if actual != expected:
            raise RuntimeError(f"pinned py65 hash mismatch: {path}")
        validated_files[relative_path] = source

    if any(name == "py65" or name.startswith("py65.") for name in sys.modules):
        raise RuntimeError("py65 was imported before pinned-source validation")

    module_plan = (
        ("py65", "py65/__init__.py", True),
        ("py65.utils", "py65/utils/__init__.py", True),
        ("py65.utils.conversions", "py65/utils/conversions.py", False),
        ("py65.utils.devices", "py65/utils/devices.py", False),
        ("py65.devices", "py65/devices/__init__.py", True),
        ("py65.devices.mpu6502", "py65/devices/mpu6502.py", False),
    )
    loaded_names: list[str] = []
    try:
        for name, relative_path, is_package in module_plan:
            module = types.ModuleType(name)
            module.__file__ = f"<pinned-py65:{relative_path}>"
            module.__package__ = name if is_package else name.rpartition(".")[0]
            if is_package:
                module.__path__ = []
            sys.modules[name] = module
            loaded_names.append(name)
            parent_name, _, child_name = name.rpartition(".")
            if parent_name:
                setattr(sys.modules[parent_name], child_name, module)
            code = compile(
                validated_files[relative_path],
                module.__file__,
                "exec",
                dont_inherit=True,
            )
            exec(code, module.__dict__)
    except Exception:
        for name in reversed(loaded_names):
            sys.modules.pop(name, None)
        raise

    expected_names = {name for name, _, _ in module_plan}
    actual_names = {
        name for name in sys.modules if name == "py65" or name.startswith("py65.")
    }
    if actual_names != expected_names:
        raise RuntimeError("pinned py65 loaded an unreviewed module")
    return sys.modules["py65.devices.mpu6502"].MPU


def generate_trace(
    mpu_type,
    image: bytes,
    expected_marker: int,
    expected_trainer_start: int,
    expected_trainer_end: int,
) -> tuple[str, int, int]:
    memory = NromMemory(image)
    mpu = mpu_type(memory=memory, pc=START)
    mpu.a = 0
    mpu.x = 0
    mpu.y = 0
    mpu.sp = 0xFD
    mpu.p = 0x24
    mpu.processorCycles = 7

    rows: list[str] = []
    for _ in range(MAX_STEPS):
        if mpu.p & 0x08:
            raise RuntimeError("clean-room trace enabled decimal mode")
        opcode = memory[mpu.pc]
        mnemonic, mode = mpu.disassemble[opcode]
        if mode not in LENGTHS:
            raise RuntimeError(f"unreviewed addressing mode {mode} at ${mpu.pc:04X}")
        length = LENGTHS[mode]
        opcode_bytes = [memory[(mpu.pc + offset) & 0xFFFF] for offset in range(length)]
        encoded = " ".join(f"{value:02X}" for value in opcode_bytes)
        rows.append(
            f"{mpu.pc:04X} {encoded} {mnemonic} "
            f"A:{mpu.a:02X} X:{mpu.x:02X} Y:{mpu.y:02X} "
            f"P:{mpu.p:02X} SP:{mpu.sp:02X} CYC:{mpu.processorCycles}"
        )
        if mpu.pc == SENTINEL:
            break
        mpu.step()
    else:
        raise RuntimeError("clean-room trace did not reach its sentinel")

    if memory.ram[0x0002] != 0x12 or memory.ram[0x07FF] != 0x46:
        raise RuntimeError("clean-room RAM mirror checks failed")
    if memory.ram[0x0100] != 0x80:
        raise RuntimeError("clean-room absolute-X page-cross setup failed")
    if memory.prg_ram[0] != 0x73 or memory.prg_ram[1] != expected_marker:
        raise RuntimeError("clean-room PRG-RAM or ROM-marker check failed")
    if (
        memory.prg_ram[2] != expected_trainer_start
        or memory.prg_ram[3] != expected_trainer_end
    ):
        raise RuntimeError("clean-room trainer endpoint checks failed")
    if memory[0x8000] != expected_marker:
        raise RuntimeError("clean-room PRG-ROM write protection failed")
    if (mpu.a, mpu.x, mpu.y, mpu.sp, mpu.pc) != (0x5A, 0x01, 0x00, 0xFD, SENTINEL):
        raise RuntimeError("clean-room final architectural state mismatch")
    return "\n".join(rows), len(rows), mpu.processorCycles


def rust_bytes(data: bytes) -> str:
    lines = []
    for offset in range(0, len(data), 16):
        chunk = ", ".join(f"0x{value:02x}" for value in data[offset : offset + 16])
        lines.append(f"    {chunk},")
    return "\n".join(lines)


def render_rust(cases: list[dict[str, object]]) -> str:
    rendered_cases = []
    for case in cases:
        rendered_cases.append(
            "    CleanroomCase {\n"
            f"        name: \"{case['name']}\",\n"
            f"        prg_banks: {case['prg_banks']},\n"
            f"        trainer: {str(case['trainer']).lower()},\n"
            f"        image_sha256: \"{case['image_sha256']}\",\n"
            f"        trace_sha256: \"{case['trace_sha256']}\",\n"
            f"        rows: {case['rows']},\n"
            f"        transitions: {case['rows'] - 1},\n"
            f"        final_cycles: {case['final_cycles']},\n"
            f"        trace: r#\"{case['trace']}\"#,\n"
            "    },"
        )
    return f'''// @generated by tools/generate-cleanroom-nrom.py; do not edit by hand.
// Oracle: py65 commit {PY65_COMMIT}; BSD-3-Clause; see NOTICE.
// Claims: architectural instruction boundaries only; bus order and reset unchecked.

const PROGRAM: &[u8] = &[
{rust_bytes(PROGRAM)}
];

const TRAINER: &[u8] = &[
{rust_bytes(TRAINER)}
];

#[derive(Clone, Copy, Debug)]
pub struct CleanroomCase {{
    pub name: &'static str,
    prg_banks: u8,
    trainer: bool,
    pub image_sha256: &'static str,
    pub trace_sha256: &'static str,
    pub rows: usize,
    pub transitions: usize,
    pub final_cycles: u64,
    pub trace: &'static str,
}}

impl CleanroomCase {{
    #[must_use]
    pub fn image(self) -> Vec<u8> {{
        let prg_len = usize::from(self.prg_banks) * 16 * 1024;
        let trainer_len = if self.trainer {{ TRAINER.len() }} else {{ 0 }};
        let mut image = vec![0; 16 + trainer_len + prg_len + 8 * 1024];
        image[0..4].copy_from_slice(b"NES\\x1a");
        image[4] = self.prg_banks;
        image[5] = 1;
        image[6] = u8::from(self.trainer) << 2;
        image[8] = 1;
        if self.trainer {{
            image[16..16 + TRAINER.len()].copy_from_slice(TRAINER);
        }}
        let prg_start = 16 + trainer_len;
        let program_offset = if self.prg_banks == 1 {{
            prg_start
        }} else {{
            prg_start + 16 * 1024
        }};
        image[program_offset..program_offset + PROGRAM.len()].copy_from_slice(PROGRAM);
        if self.prg_banks == 2 {{
            image[prg_start] = 0x5a;
        }}
        let vector = prg_start + prg_len - 4;
        image[vector..vector + 2].copy_from_slice(&[0x00, 0xc0]);
        image
    }}
}}

pub const PY65_COMMIT: &str = "{PY65_COMMIT}";
pub const PY65_CPU_SHA256: &str =
    "{PY65_CPU_SHA256}";
pub const CASES: &[CleanroomCase] = &[
{chr(10).join(rendered_cases)}
];
'''


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--py65-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    mpu_type = require_pinned_py65(args.py65_root.resolve())
    cases = []
    case_specs = (
        ("nrom128", 1, False, 0xD8, 0x00, 0x00),
        ("nrom256", 2, False, 0x5A, 0x00, 0x00),
        ("nrom128_trainer", 1, True, 0xD8, TRAINER[0], TRAINER[-1]),
    )
    for name, prg_banks, trainer, marker, trainer_start, trainer_end in case_specs:
        image = build_image(prg_banks, trainer)
        trace, rows, final_cycles = generate_trace(
            mpu_type, image, marker, trainer_start, trainer_end
        )
        cases.append(
            {
                "name": name,
                "prg_banks": prg_banks,
                "trainer": trainer,
                "image_sha256": sha256(image),
                "trace_sha256": sha256(trace.encode("ascii")),
                "rows": rows,
                "final_cycles": final_cycles,
                "trace": trace,
            }
        )

    output = render_rust(cases)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(output, encoding="utf-8", newline="\n")
    print(f"wrote {len(cases)} clean-room cases to {args.output}")
    print(f"generated_sha256={sha256(output.encode('utf-8'))}")


if __name__ == "__main__":
    main()
