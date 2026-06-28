let count = 0;
let rootEl = document.createElement('div');
rootEl.id = 'root';
document.body.appendChild(rootEl);
let root = ReactDOM.createRoot(rootEl);

function renderApp() {
  let label = 'Count: ' + count;
  let btn = React.createElement('button', {
    className: 'counter-btn',
    onClick: () => {
      count = count + 1;
      renderApp();
    }
  }, label);
  root.render(btn);
}

renderApp();
