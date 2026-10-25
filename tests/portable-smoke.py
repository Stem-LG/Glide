"""Run under a private dbus-run-session + Xvfb; never emit desktop input."""
import os,pathlib,re,socket,subprocess,sys,tempfile,time,urllib.request
binary=str(pathlib.Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory(prefix='glide-portable-test-') as config:
 with socket.socket() as sock:
  sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 env={**os.environ,'GLIDE_CONFIG_DIR':config,'GDK_BACKEND':'x11','GTK_A11Y':'none'}
 with tempfile.TemporaryFile(mode='w+') as log:
  proc=subprocess.Popen([binary,'--preview','--port',str(port)],env=env,stdout=log,stderr=log)
  try:
   for _ in range(150):
    if proc.poll() is not None:raise AssertionError(f'GUI exited: {proc.returncode}')
    try:
     with urllib.request.urlopen(f'http://127.0.0.1:{port}/health',timeout=.3) as response:
      assert b'Glide' in response.read()
     tree=subprocess.check_output(['xwininfo','-root','-tree'],text=True)
     if re.search(r'"Glide"',tree):break
    except (OSError,subprocess.CalledProcessError):pass
    time.sleep(.1)
   else:raise AssertionError('No live Glide window and HTTP server')
   time.sleep(2)
   assert proc.poll() is None
   log.seek(0);output=log.read()
   assert not any(s in output for s in ['symbol lookup error','error while loading shared libraries','GLib-GObject-CRITICAL','Gtk-CRITICAL']),output
   if os.environ.get('GLIDE_SMOKE_SCREENSHOT'):
    subprocess.run(['xwd','-name','Glide','-silent','-out',os.environ['GLIDE_SMOKE_SCREENSHOT']],check=True)
   print('PASS: packaged GTK window, HTTP health, and stable startup')
   if output:print(output)
  finally:
   proc.terminate()
   try:proc.wait(timeout=5)
   except subprocess.TimeoutExpired:proc.kill();proc.wait()
