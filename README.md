# hexfetch
Fetching HEX Pulsechain API with Golang


## Usage

`hexfetch-cli`     - This will use 1 T-Share  
`hexfetch-cli 100` - This will use 100 T-Shares  

Alternatively, add alias into .bashrc: `alias hexfetch='$HOME/hexfetch <T-Shares>'`  
Use new .bashrc: `source ~/.bashrc`  
Run `hexfetch`

Output:
```
HEX Price      : 0.004128 $
T-Share Price  : 149.92 $
T-Share Rate   : 36321.7 HEX
T-Share Payout : 753.181 HEX
T-Share Value  : 14992.00 $
T-Shares       : 100.00
```
If there are changes, it will output the changed values:
```
HEX Price      : 0.004129 $ (+0.000002)
T-Share Price  : 149.96 $ (+0.056300)
T-Share Rate   : 36321.7 HEX
T-Share Payout : 753.181 HEX
T-Share Value  : 14996.00 $ (+4.000000)
T-Shares       : 100.00
```
