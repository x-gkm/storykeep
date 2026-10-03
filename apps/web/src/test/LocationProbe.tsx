import { useLocation } from 'react-router'

/** Renders the current route so tests can assert on navigation. */
export function LocationProbe() {
  const location = useLocation()
  return <div data-testid="location">{location.pathname + location.search}</div>
}
