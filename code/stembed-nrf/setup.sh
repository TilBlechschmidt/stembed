# Before first usage, you need to disable the flash protection by erasing everything in it
# https://blog.dbrgn.ch/2020/5/16/nrf52-unprotect-flash-jlink-openocd/
# 1. Plug the nRF52 into power
# 2. Plug the attached J-Tag into power
# 3. Run the following command
openocd -c 'interface jlink; transport select swd; source [find target/nrf52.cfg]'
# 4. Open a second terminal and use telnet
# telnet localhost 4444
# 5. Use the following to check whether flash protection is disabled
# nrf52.dap apreg 1 0x0c
# 1 means it is disabled while just zeros mean it is enabled
