import init, * as soc from '../soc/pkg/soc.js';

// Some globals.
var simulator;

// Store the last time update_tick was called.
var last_time;

function start_from_bytes(cart_bytes) {
  simulator = soc.Simulator.from_cart_bytes(new Uint8Array(cart_bytes));
  last_time = performance.now();
  window.requestAnimationFrame(update_tick);
}

function update_tick(new_time) {
  var dt = (new_time - last_time) / 1000.0;
  last_time = new_time;
  // dt can be 0 if update_tick is called multiple times per frame.
  if (dt > 0) {
    // var t = performance.now();
    var maybe_data = simulator.update(dt);
    // console.log(performance.now() - t);
    // If we have a new screen, put it into backing_image.
    if (maybe_data) {
      var backing_image = document.getElementById('backing_image');
      {
        var backing_ctx = backing_image.getContext('2d');
        var imageData = backing_ctx.getImageData(0, 0, 160, 144);
        var data = imageData.data;
        for (var i = 0; i < data.length; i++) {
          data[i] = maybe_data[i];
        }
        backing_ctx.putImageData(imageData, 0, 0);
      }
    }
  }
  window.requestAnimationFrame(update_tick);
}

function handleFileChange() {
  const file_button = document.getElementById('file_button');
  if (this.files.length == 0) {
    file_button.innerHTML = 'File';
    return;
  }
  file_button.innerHTML = 'Loading...';
  const reader = new FileReader();
  reader.onload = function(e) {
    console.log(e.target);
    start_from_bytes(e.target.result);
  };
  reader.readAsArrayBuffer(this.files[0]);
}

export async function run() {
  await init();

  document.getElementById('cart_file')
      .addEventListener('change', handleFileChange, false);
}

run();