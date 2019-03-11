window.addEventListener('load', evt => {
  let storedIndex = sessionStorage.getItem('index');
  let index = storedIndex === null ? 0 : parseInt(storedIndex);
  const slides = document.getElementsByClassName('slide');

  function update() {
    for (let i = 0; i < slides.length; i++) {
      const slide = slides.item(i);
      slide.classList.remove('previous');
      slide.classList.remove('current');
      slide.classList.remove('next');
      if (i === index - 1) {
        slide.classList.add('previous');
      } else if (i === index) {
        slide.classList.add('current');
      } else if (i === index + 1) {
        slide.classList.add('next');
      }
    }
    sessionStorage.setItem('index', index);
  }

  update();

  // Handle key events
  window.addEventListener('keydown', evt => {
    if (evt.key === 'ArrowLeft') {
      if (index === 0) {
        return;
      }
      index--;
      update();
    } else if (evt.key === 'ArrowRight') {
      if (index == slides.length - 1) {
        return;
      }
      index++;
      update();
    }
  }, false);

  const query = new URLSearchParams(window.location.search);
  if (query.get("watch") == "true") {
    // Setup auto-reload using a websocket transport
    const uri = 'ws://' + window.location.host + '/ws';
    const ws = new WebSocket(uri);
    ws.onopen = () => {
      console.log('[WS] Connected');
    };
    ws.onmessage = msg => {
      console.log('[WS] Message', msg);
      const event = JSON.parse(msg.data);
      if (event.type === 'reload') {
        window.location.reload();
      }
    };
  }
}, false)
