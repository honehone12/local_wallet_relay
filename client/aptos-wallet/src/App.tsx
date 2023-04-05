import React from 'react';
import './App.css';

interface RpcRequest {
  type: string;
  function: string;
  arguments: string[];
  type_arguments: string[];
}

function App() {
  const eventSource = new EventSource('sse');
  const [address, setAddress] = React.useState<string | null>(null);

  const init = async () => {
    eventSource.addEventListener('data', function(event) {  
      console.log('server sent event: ', event.data);
      const payload: RpcRequest = JSON.parse(event.data);
      eventSource.close();
    });
    const {address} = await window.aptos.connect();
    setAddress(address);
  }

  React.useEffect(() => {
    init();
  }, []);

  return (
    <div className="App">
      <header className="App-header">
        <p>Connecting to wallet <code>{address}</code></p>
      </header>
    </div>
  );
}

export default App;
