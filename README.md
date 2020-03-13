## Generating logs for obj-loc

- Record the execution in rr because the logs will change in each run.
- Start rr without a .gdbinit:

  ```
  $ rr replay -- --nx
  ```
- You'll need to make a full run before adding breakpoints as otherwise gdb/rr
  can't find locations of the symbols/files. Just do `run`.
- Update `gdb.txt` below with the file location you like.
- Add a breakpoint to places where the GC moves an object. Similarly print at
  the beginning of each GC whether it's a major GC or not. Example:
  ```
  set pagination off
  set logging file gdb.txt
  set logging redirect on
  set logging on
  break GC.c:269
  commands 1
  printf ">>> GC %d\n", major_gc
  continue
  end
  break move
  commands 2
  printf ">>> %p -> %p size: %d\n", from, to, size
  continue
  end
  break Evac.c:148
  commands 3
  printf ">>> %p -> %p size: %d\n", from, to, size
  continue
  end
  ```
  **Note that gdb by default extends the log file, does not override it! Make
  sure to use a new log file every time (or remove the old one before recording
  a new one!)**
