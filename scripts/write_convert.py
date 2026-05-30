import pathlib,base64,sys

data = sys.stdin.buffer.read()
pathlib.Path(sys.argv[1]).write_bytes(base64.b64decode(data))
print('OK')
