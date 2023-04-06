import React from 'react';
import './App.css';

interface RpcRequest {
  type: string;
  function: string;
  arguments: string[];
  type_arguments: string[];
}

interface Address {
  hex: string
}

function App() {
  let eventSource: EventSource | null = null;
  const [address, setAddress] = React.useState<string | null>(null);
  const params = new URLSearchParams(window.location.search);

  const init = async () => {
    const {address} = await window.aptos.connect();
    setAddress(address);
    
    if (params.has('payload')) {
      eventSource = new EventSource('sse');
      if (eventSource !== null) {
        eventSource.addEventListener('payload', async (event) => {  
          console.log('server sent event: ', event.data);
          if (eventSource !== null) {
            eventSource.close();
          }
          const payload: RpcRequest = JSON.parse(event.data);
          try {
            await window.aptos.signAndSubmitTransaction(payload);
          } catch (e) {
            console.log(e);
          }
        });
      }
    }

    if (params.has('address') && address !== null) {
      try {
        const res = await fetch('http://127.0.0.1:8080/address', {
          method: 'POST',
          mode: 'same-origin',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({hex: address}) 
        });
        console.log('response of fetch: ', res);
      } catch (e) {
        console.log(e);
      }
    }
  }

  React.useEffect(() => {
    init();
  }, []);

  return (
    <div className="App">
      <header className="App-header">
        <p>
          This is served from local computer.<br/>
          Only your wallet has internet access.
        </p>
      </header>
    </div>
  );
}

export default App;
