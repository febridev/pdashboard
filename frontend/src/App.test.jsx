import { it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import App from './App'

it('renders the dashboard shell', () => {
  render(<App />)
  expect(screen.getByText('Personal Dashboard')).toBeInTheDocument()
})
