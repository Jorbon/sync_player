import socket

s = socket.socket()
s.bind(("", 7777))
s.setblocking(0)
s.listen()

connections = []

while True:
	try:
		conn, addr = s.accept()
		print(f"Connected to {addr}")
		connections.append(conn)
	except BlockingIOError:
		pass
	
	for conn in connections:
		try:
			data = conn.recv(256)
			if data:
				print(data)
		except ConnectionResetError:
			print(f"{conn.getpeername()} disconnected")
			connections.remove(conn)
			continue
		except BlockingIOError:
			pass
		
		try:
			conn.sendall(b"ping from server")
		except ConnectionResetError:
			print(f"{conn.getpeername()} disconnected")
			connections.remove(conn)
			continue
	