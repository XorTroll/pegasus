# pegasus

> Work-in-progress Nintendo Switch emulator, written in pure Rust and slightly less focused on gaming

## Information

- This project aims to be a different emulator, not one mostly focused on gaming but slightly more inclined on the developer path. However, it is still in a quite early stage, so it's aim may or may not eventually change.

- This project has a semi-HLE design goal: the only components which will be implemented in the emulator itself are the kernel and sysmodules (those where getting the actual component to work would likely be more complicated than actually implementing them myself)

- Unlike other emulators which are built around the idea of just launching whatever they get opened with, this project aims to emulate an actual console.

  - This is why the project requires system titles to be placed inside the NAND like an actual console, since it will launch them (HOME menu, etc.) almost like a normal console would, excluding emulated components.

## Config and keys

When launched, if not already present, pegasus will create a default config file, default NAND/SD directories, all of them on the current working directory. It will then look for a `prod.keys` in the same directory, panicking if the keyset is invalid or not present.

### Config file

The config file, `config.cfg`, is a JSON file with the following fields:

| Field            | Type   | Default value (when created) | Description                                                         |
|------------------|--------|------------------------------|---------------------------------------------------------------------|
| nand_system_path | string | {cwd}/nand_system            | NAND system path, where system titles are located                   |
| nand_user_path   | string | {cwd}/nand_user              | NAND user path (where titles installed on console will be located?) |
| sd_card_path     | string | {cwd}/sd_card                | SD card path                                                        |

## Source layout

Since this ain't a small project, some guidelines about how this project's source code is structured:

- Everything emulation-specific goes inside `emu` module.

- All emulated processes go inside `proc::<proc-name>` module while they have their types at `<proc-name>` module (since other processes may use them), and use submodules for any IPC interfaces they have.

- Results for a certain module are placed in `<module>::result` and all follow a similar format, using a macro to define them.

- Due to some questionable design thoughts on the official Unicorn Rust bindings, this project makes use of a custom version (see [unicorn-rs](unicorn-rs))

## Credits

- [Unicorn](https://github.com/unicorn-engine/unicorn) as the CPU engine for this emulator, basically doing the hard job of this kind of project.

- [Ryujinx](https://github.com/Ryujinx/Ryujinx) was mostly the base for this project - most of the kernel was implemented based on Ryujinx's, and several design thoughts for this emulator came from its design.

- [Atmosphere](https://github.com/Atmosphere-NX/Atmosphere)'s kernel and sm reimplementations were really helpful for this project's implementations.

- [cntx](https://github.com/XorTroll/cntx) libraries (guess who coded them) for exploring several formats (NCA, PFS0, etc.)

## TODO

- Make logging slightly more verbose, maybe add a log message to the result error?

- Memory support in kernel (+ memory SVCs)

- Buffer support in kernel IPC code (which needs kernel memory support)

- Keep implementing more processes and services

- The various `todo!`s left here and there in the code