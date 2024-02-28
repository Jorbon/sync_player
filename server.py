import socket

s = socket.socket()
s.bind(("", 7777))
s.setblocking(0)
s.listen()

connections = []

while True:
	try:
		conn, addr = s.accept()
		connections.append([conn, addr, True, []])
		print(f"Connected to {addr}, {len(connections)} connections in total")
	except BlockingIOError:
		pass
	
	for i in range(len(connections)):
		if connections[i][2] == False:
			continue
		
		try:
			while True:
				try:
					connections[i][3].append(connections[i][0].recv(1))
				except (ConnectionResetError, ConnectionAbortedError):
					connections[i][2] = False
					break
				except BlockingIOError:
					break
			
			if connections[i][2] == False:
				continue
			
			data_len = len(connections[i][3]) - 2
			if data_len >= 0:
				message_len = int.from_bytes(b"".join(connections[i][3][:2]), "little")
				if data_len >= message_len:
					
					data = b"".join(connections[i][3][2:])
					
					if data == b"exit":
						connections[i][2] = False
						continue
					
					connections[i][3] = connections[i][3][message_len + 2:]
					print(f"Sending {data}")
					
					for j in range(len(connections)):
						if i != j and connections[j][2] == True:
							try:
								connections[j][0].sendall(data)
							except (ConnectionResetError, ConnectionAbortedError):
								connections[j][2] = False
								continue
							except BlockingIOError:
								pass
			
		except (ConnectionResetError, ConnectionAbortedError):
			connections[i][2] = False
			continue
	
	for i in range(len(connections))[::-1]:
		if connections[i][2] == False:
			connections[i][0].close()
			print(f"{connections[i][1]} disconnected, {len(connections) - 1} connections in total")
			connections.pop(i)
			break
	