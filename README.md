# RWT

Utility for reading and writing tests.

## Usage

```text
Usage: rwt.exe [OPTIONS]

Options:
  -i, --input <INPUT>
          Input file
  -o, --output <OUTPUT>
          Output file. Output to memory by default.
  -g, --generator <CONTENT>
          Generate output content.
          If it is random type, all generated into memory first;
          and if count is 0, memory size only is buffer size.
           [possible values: text, null, random, random-text]
  -b, --buffer-size <BUFFER_SIZE>
          Buffer size, like:
          1 KiB = 1 Ki = 1024 Bytes
          1 KB  = 1 K  = 1000 Bytes
           [default: 4KiB]
  -c, --count <COUNT>
          Buffer count.
          0: Read and write until EOF or SIGINT.
           [default: 0]
      --completion <SHELL>
          Print shell completion script
           [possible values: bash, elvish, fish, powershell, zsh]
  -v, --version
          Print version
  -h, --help
          Print help
```

### Copy File (Read & Write)

```text
> rwt -i 1 -o 2 -b 4MiB
Buffer size: 4194304 Byte (4 MiB, 4.194304 MB)
RW duration: 620.5277ms
RW count: 256
RW size: 1073741824 Byte (1 GiB, 1.073741824 GB)
RW speed: 1.61 GiB/s, 1.73 GB/s, 12.89 Gib/s, 13.84 Gb/s
```
### Read File

```text
> rwt -i 1 -b 4MiB
Buffer size: 4194304 Byte (4 MiB, 4.194304 MB)
RW duration: 204.148ms
RW count: 256
RW size: 1073741824 Byte (1 GiB, 1.073741824 GB)
RW speed: 4.9 GiB/s, 5.26 GB/s, 39.19 Gib/s, 42.08 Gb/s
```

### Write File with Generator

```text
> rwt -g random -o 2 -b 4MiB -c 1
Buffer size: 4194304 Byte (4 MiB, 4.194304 MB)
Generating into memory, size: 4194304 Byte (4 MiB, 4.194304 MB)
Generation duration: 53.0788ms
Generation speed: 75.36 MiB/s, 79.02 MB/s, 602.88 Mib/s, 632.16 Mb/s
RW duration: 2.1293ms
RW count: 1
RW size: 4194304 Byte (4 MiB, 4.194304 MB)
RW speed: 1.83 GiB/s, 1.97 GB/s, 14.68 Gib/s, 15.76 Gb/s
```
