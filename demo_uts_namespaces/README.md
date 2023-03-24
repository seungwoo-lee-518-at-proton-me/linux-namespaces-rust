# `demo_uts_namespaces`

* Requires privilege to use `CLONE_NEWUTS`.

## Demo

```bash
# ./demo_uts_namespaces
process has been created as pid 48648
set hostname as hellowed                <== Child
get hostname                            
hostname is: hellowed
parent hostname is: DESKTOP-K99IERK     <== Parent
child has terminated                    <== Wait for Child has been Terminated
```
