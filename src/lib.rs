const SCREEN_WIDTH: u8 = 64;
const SCREEN_HEIGHT: u8 = 32;

pub struct Chip8 {
    cpu: CPU,
    display: Display,
    memory: Memory,
}

const ROM_START: usize = 512;

impl Chip8 {
    pub fn new(rom: &[u8]) -> Chip8 {
        let mut memory = Memory::new();
        memory.0[ROM_START..(ROM_START + rom.len())].copy_from_slice(rom);
        Chip8 {
            cpu: CPU {
                delay_timer: 0,
                sound_timer: 0,
                stack: Vec::new(),
                program_counter: ROM_START as u16,
                index_register: 0,
                registers: [0; 16],
            },
            display: Display::new(),
            memory,
        }
    }

    pub fn advance(&mut self) {
        let Chip8 {
            cpu,
            display,
            memory,
        } = self;
        advance(cpu, display, memory);
    }

    pub fn display(&self) -> &Display {
        &self.display
    }
}

pub fn advance(cpu: &mut CPU, display: &mut Display, memory: &mut Memory) {
    let instr = [
        memory.get(cpu.program_counter),
        memory.get(cpu.program_counter + 1),
    ];
    let instr = ((instr[0] as u16) << 8) | instr[1] as u16;
    cpu.program_counter += 2;

    if cpu.delay_timer > 0 {
        cpu.delay_timer -= 1;
    }
    if cpu.sound_timer > 0 {
        cpu.sound_timer -= 1;
    }

    // Match on first nibble
    match instr >> 12 {
        0 => match instr & 0x00FF {
            0xE0 => {
                display.clear();
            }
            0xEE => {
                // return
                cpu.program_counter = cpu.stack.pop().expect("Tried to return but stack empty");
            }
            _ => panic!("instruction unknown: {}", instr),
        },
        1 => {
            let target = instr & 0x0FFF;
            cpu.program_counter = target;
        }
        2 => {
            // "subroutine"
            let target = instr & 0x0FFF;
            cpu.stack.push(cpu.program_counter);
            cpu.program_counter = target;
        }
        3 => {
            // skip if val equal
            let register_value = cpu.register((instr & 0x0F00) >> 8);
            let value = (instr & 0x00FF) as u8;
            if register_value == value {
                cpu.program_counter += 2;
            }
        }
        4 => {
            // skip if val not equal
            let register_value = cpu.register((instr & 0x0F00) >> 8);
            let value = (instr & 0x00FF) as u8;
            if register_value != value {
                cpu.program_counter += 2;
            }
        }
        5 => {
            // skip if register equal
            let a = cpu.register((instr & 0x0F00) >> 8);
            let b = cpu.register((instr & 0x00F0) >> 4);
            if a == b {
                cpu.program_counter += 2;
            }
        }
        9 => {
            // skip if register not equal
            let a = cpu.register((instr & 0x0F00) >> 8);
            let b = cpu.register((instr & 0x00F0) >> 4);
            if a != b {
                cpu.program_counter += 2;
            }
        }
        6 => {
            cpu.set_register((instr & 0x0F00) >> 8, (instr & 0x00FF) as u8);
        }
        7 => {
            // add to register
            let index = (instr & 0x0F00) >> 8;
            let register_value = cpu.register(index);
            let constant = instr & 0x00FF;
            let sum = (register_value as u16) + constant;
            cpu.set_register(index, sum as u8);
        }
        8 => {
            // logical and arithmetic operators
            let r = (instr & 0x0F00) >> 8;
            let a = cpu.register(r);
            let b = cpu.register((instr & 0x00F0) >> 4);
            match instr & 0x000F {
                0 => {
                    cpu.set_register(r, b);
                }
                1 => {
                    cpu.set_register(r, a | b);
                }
                2 => {
                    cpu.set_register(r, a & b);
                }
                3 => {
                    cpu.set_register(r, a ^ b);
                }
                4 => {
                    let result = (a as u16) + (b as u16);
                    cpu.set_register(r, result as u8);
                    cpu.set_register(0xF, if result > 255 { 1 } else { 0 });
                }
                5 | 7 => {
                    let result = if (instr & 0x000F) == 5 {
                        (a as i16) - (b as i16)
                    } else {
                        (b as i16) - (a as i16)
                    };
                    if dbg!(result) >= 0 {
                        cpu.set_register(r, result as u8);
                        cpu.set_register(0xF, 1);
                    } else {
                        cpu.set_register(r, (256 + result) as u8);
                        cpu.set_register(0xF, 0);
                    }
                }
                6 => {
                    cpu.set_register(r, a >> 1);
                }
                0xE => {
                    cpu.set_register(r, a << 1);
                }
                _ => panic!("instruction unknown: {}", instr),
            }
        }
        0xA => {
            cpu.index_register = instr & 0x0FFF;
        }
        0xB => {
            // jump with offset
            // quirk?
            let pointer = cpu.register(0) as u16;
            let offset = instr & 0x0FFF;
            cpu.program_counter = pointer + offset;
        }
        0xC => {
            // random
            let mask = (instr & 0x00FF) as u8;
            let rand_value = rand::random::<u8>();
            cpu.set_register((instr & 0x0F00) >> 8, mask & rand_value);
        }
        0xD => {
            // display
            let x = cpu.register((instr & 0x0F00) >> 8) % SCREEN_WIDTH;
            let mut y = cpu.register((instr & 0x00F0) >> 4) % SCREEN_HEIGHT;
            let n = instr & 0x00F;
            cpu.set_register(0xF, 0);
            for i in 0..n {
                let byte = memory.get(i + cpu.index_register);
                let mut x = x;
                for bit in 0..7 {
                    let is_on = (byte & (0b10000000u8 >> bit)) != 0;
                    if is_on {
                        let pixel = display.get_pixel_mut(x, y);
                        if *pixel {
                            cpu.set_register(0xF, 1);
                        }
                        *pixel = !*pixel;
                    }
                    x += 1;
                    if x >= SCREEN_WIDTH {
                        continue;
                    }
                }
                y += 1;
                if y >= SCREEN_HEIGHT {
                    continue;
                }
            }
        }
        0xE => {
            // skip based on input
            // TODO: keyboard input
        }
        0xF => {
            let op = (instr & 0x0F00) >> 8;
            match instr & 0x00FF {
                0x07 => {
                    cpu.set_register(op, cpu.delay_timer);
                }
                0x15 => {
                    cpu.delay_timer = cpu.register(op);
                }
                0x18 => {
                    cpu.sound_timer = cpu.register(op);
                }
                0x1E => {
                    let result = (cpu.register(op) as u16) + cpu.index_register;
                    if result > 0x0FFF {
                        cpu.set_register(0xF, 1);
                    }
                    cpu.index_register = result;
                }
                0x0A => {
                    // TODO: block and wait for key input
                }
                0x29 => {
                    cpu.index_register = (FONT_START as u16) + (cpu.register(op) as u16) & 0x0F * 5;
                }
                0x33 => {
                    let value = cpu.register(op);
                    memory.set(cpu.index_register + 2, value % 10);
                    memory.set(cpu.index_register + 1, (value / 10) % 10);
                    memory.set(cpu.index_register, (value / 100) % 10);
                }
                0x55 => {
                    for i in 0..=op {
                        memory.set(cpu.index_register + i, cpu.register(i));
                    }
                    // quirk?
                }
                0x65 => {
                    for i in 0..=op {
                        cpu.set_register(i, memory.get(cpu.index_register + i));
                    }
                    // quirk?
                }
                _ => panic!("unknown instruction: {}", instr),
            }
        }
        _ => unreachable!("Cannot have a nibble higher than 0xF"),
    }
}

pub struct CPU {
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub stack: Vec<u16>,
    pub program_counter: u16,
    pub index_register: u16,
    pub registers: [u8; 16],
}

impl CPU {
    pub fn register(&self, index: u16) -> u8 {
        self.registers[index as usize]
    }

    pub fn set_register(&mut self, index: u16, value: u8) {
        self.registers[index as usize] = value;
    }
}

pub struct Display(Box<[[bool; 64]; 32]>);

impl Display {
    pub fn new() -> Display {
        Display(Box::new([[false; 64]; 32]))
    }

    pub fn clear(&mut self) {
        for row in self.0.iter_mut() {
            for cell in row.iter_mut() {
                *cell = false;
            }
        }
    }

    pub fn get_pixel(&self, x: u8, y: u8) -> bool {
        self.0[y as usize][x as usize]
    }

    pub fn get_pixel_mut(&mut self, x: u8, y: u8) -> &mut bool {
        &mut self.0[y as usize][x as usize]
    }
}

const FONT_START: usize = 0x50;

pub struct Memory(Box<[u8; 4096]>);

impl Memory {
    pub fn new() -> Memory {
        let mut buffer = Box::new([0u8; 4096]);

        let font = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];

        buffer[FONT_START..(font.len() + FONT_START)].copy_from_slice(&font);

        Memory(buffer)
    }

    pub fn get(&self, idx: u16) -> u8 {
        self.0[idx as usize]
    }

    pub fn set(&mut self, idx: u16, value: u8) {
        self.0[idx as usize] = value;
    }
}
